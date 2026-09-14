//! Tests for the SDK's [`PythLazerClient::verify_update`] API. These live in
//! `integration-tests` (rather than the SDK crate) because they need the
//! deployed on-chain verifier contract, which the SDK does not depend on.
//!
//! A small consumer contract wraps the SDK call so the test invokes it via a
//! generated Client — matching the real integration pattern (consumer contract
//! ↦ SDK ↦ on-chain verifier).

extern crate alloc;

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN,
    Env, IntoVal,
};

use pyth_lazer_stellar::{PythLazerContract, PythLazerContractClient};
use pyth_lazer_stellar_sdk::{PythLazerClient, VerifyError};

/// Consumer contract that forwards to [`PythLazerClient::verify_update`] so
/// tests can call it via a generated `try_verify` and observe the SDK's
/// [`VerifyError`] as a typed contract error.
#[contract]
struct SdkConsumer;

#[contractimpl]
impl SdkConsumer {
    pub fn verify(env: Env, lazer: Address, data: Bytes) -> Result<u32, VerifyError> {
        let client = PythLazerClient::new(&env, &lazer);
        let verified = client.verify_update(&data)?;
        Ok(verified.feeds.len() as u32)
    }
}

fn test_lazer_update_bytes(env: &Env) -> Bytes {
    Bytes::from_slice(
        env,
        &hex_literal::hex!(
            "e4bd474d73a7e70a8e2b8de236b55dcc6a771b4a8a1533fe"
            "492f424fae162369fa14103e04c1c93302cef8a052110a95"
            "0da031f9dc5eade9e6099e95668aff2592ec1f7900fe0075"
            "d3c7934067e9c7f14a06000303010000000b00e1637ad535"
            "060000015a2507d335060000027f8bfdf53506000004f8ff"
            "0600070008000900000a601299cd3e0600000bc07595c73e"
            "0600000c014067e9c7f14a0600020000000b00971b209c2d"
            "0000000144056b9b2d0000000298fb6b9c2d00000004f8ff"
            "0600070008000900000a284444f92d0000000b480c07f92d"
            "0000000c014067e9c7f14a0600700000000b0020d85dd2d7"
            "8df30001000000000000000002000000000000000004f4ff"
            "060130f80bfeffffffff0701b8ab7057ec4a060008010020"
            "9db4060000000900000a00000000000000000b0000000000"
            "0000000c014067e9c7f14a0600"
        ),
    )
}

fn test_trusted_signer_pubkey(env: &Env) -> BytesN<33> {
    BytesN::from_array(
        env,
        &hex_literal::hex!("03a4380f01136eb2640f90c17e1e319e02bbafbeef2e6e67dc48af53f9827e155b"),
    )
}

struct Fixture<'a> {
    env: Env,
    lazer: PythLazerContractClient<'a>,
    consumer: SdkConsumerClient<'a>,
    executor: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    let executor = Address::generate(&env);
    let lazer_id = env.register(
        PythLazerContract,
        (executor.clone(), None::<BytesN<33>>, None::<u64>),
    );
    let consumer_id = env.register(SdkConsumer, ());
    Fixture {
        lazer: PythLazerContractClient::new(&env, &lazer_id),
        consumer: SdkConsumerClient::new(&env, &consumer_id),
        env,
        executor,
    }
}

fn add_trusted_signer(fx: &Fixture, pubkey: &BytesN<33>, expires_at: u64) {
    fx.lazer
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &fx.executor,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &fx.lazer.address,
                fn_name: "update_trusted_signer",
                args: (pubkey.clone(), expires_at).into_val(&fx.env),
                sub_invokes: &[],
            },
        }])
        .update_trusted_signer(pubkey, &expires_at);
}

fn verify_via_sdk(fx: &Fixture, update: &Bytes) -> Result<u32, VerifyError> {
    match fx.consumer.try_verify(&fx.lazer.address, update) {
        Ok(Ok(n)) => Ok(n),
        Err(Ok(e)) => Err(e),
        // The SDK translates every verifier-side failure into a typed
        // VerifyError, so decode failures and untyped invocation errors are
        // not expected in these tests.
        Ok(Err(decode_err)) => panic!("unexpected decode failure: {:?}", decode_err),
        Err(Err(invoke_err)) => panic!("unexpected invoke failure: {:?}", invoke_err),
    }
}

#[test]
fn verify_update_success_returns_verified_payload() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 2_000_000_000);

    let update = test_lazer_update_bytes(&fx.env);
    let feed_count = verify_via_sdk(&fx, &update).expect("verify_update should succeed");
    assert_eq!(feed_count, 3);
}

#[test]
fn verify_update_untrusted_signer_returns_err() {
    let fx = setup();
    let wrong_pubkey = BytesN::from_array(
        &fx.env,
        &hex_literal::hex!("03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );
    add_trusted_signer(&fx, &wrong_pubkey, 2_000_000_000);

    let update = test_lazer_update_bytes(&fx.env);
    assert_eq!(
        verify_via_sdk(&fx, &update),
        Err(VerifyError::SignerNotTrusted)
    );
}

#[test]
fn verify_update_expired_signer_returns_err() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 1_000);
    fx.env.ledger().with_mut(|li| li.timestamp = 1_000);

    let update = test_lazer_update_bytes(&fx.env);
    assert_eq!(
        verify_via_sdk(&fx, &update),
        Err(VerifyError::SignerExpired)
    );
}

#[test]
fn verify_update_invalid_envelope_magic_returns_err() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 2_000_000_000);

    let mut update_raw = test_lazer_update_bytes(&fx.env).to_alloc_vec();
    update_raw[0] = 0xFF;
    let update = Bytes::from_slice(&fx.env, &update_raw);
    assert_eq!(
        verify_via_sdk(&fx, &update),
        Err(VerifyError::InvalidEnvelopeMagic)
    );
}

#[test]
fn verify_update_truncated_envelope_returns_err() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 2_000_000_000);

    let update = Bytes::from_slice(&fx.env, &[0u8; 50]);
    assert_eq!(
        verify_via_sdk(&fx, &update),
        Err(VerifyError::TruncatedEnvelope)
    );
}

#[test]
fn verify_update_invalid_recovery_id_returns_err() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 2_000_000_000);

    let mut update_raw = test_lazer_update_bytes(&fx.env).to_alloc_vec();
    update_raw[68] = 0xFF;
    let update = Bytes::from_slice(&fx.env, &update_raw);
    assert_eq!(
        verify_via_sdk(&fx, &update),
        Err(VerifyError::InvalidRecoveryId)
    );
}

#[test]
fn verify_update_invalid_envelope_length_returns_err() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 2_000_000_000);

    let mut update_raw = test_lazer_update_bytes(&fx.env).to_alloc_vec();
    update_raw[69] = 0xFF;
    let update = Bytes::from_slice(&fx.env, &update_raw);
    assert_eq!(
        verify_via_sdk(&fx, &update),
        Err(VerifyError::InvalidEnvelopeLength)
    );
}

/// Invalid-signature envelope (bit-flip in the signature). The host's
/// `secp256k1_recover` may either produce an unknown pubkey (→ SignerNotTrusted)
/// or trap on non-canonical inputs (→ InvokeFailed). Either surfaces as a
/// typed `Err` rather than panicking through the SDK client.
#[test]
fn verify_update_invalid_signature_returns_err() {
    let fx = setup();
    add_trusted_signer(&fx, &test_trusted_signer_pubkey(&fx.env), 2_000_000_000);

    let mut update_raw = test_lazer_update_bytes(&fx.env).to_alloc_vec();
    // Flip a byte in the middle of the signature (offset 4..68 is signature).
    update_raw[10] ^= 0xFF;
    let update = Bytes::from_slice(&fx.env, &update_raw);

    let err = verify_via_sdk(&fx, &update).expect_err("bad signature must not verify");
    assert!(
        matches!(
            err,
            VerifyError::SignerNotTrusted | VerifyError::InvokeFailed
        ),
        "unexpected error variant: {:?}",
        err
    );
}
