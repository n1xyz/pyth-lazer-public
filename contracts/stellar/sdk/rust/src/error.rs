use soroban_sdk::contracterror;

/// Errors returned by [`crate::parse_payload`] when decoding a verified Lazer
/// payload. Declared as a `#[contracterror]` so consumers can propagate it
/// directly from their own contract entrypoints.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ParseError {
    TruncatedData = 1,
    InvalidPayloadLength = 2,
    InvalidPayloadMagic = 3,
    InvalidChannel = 4,
    InvalidProperty = 5,
    InvalidMarketSession = 6,
}

/// Errors returned by [`crate::PythLazerClient::verify_update`].
///
/// Covers both verifier-side failures reported by the on-chain
/// `pyth-lazer-stellar` contract and parse failures on the returned payload
/// bytes. The verifier-side discriminants match the on-chain contract's error
/// codes so [`soroban_sdk::Env::try_invoke_contract`] converts them directly.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VerifyError {
    /// Envelope magic prefix did not match the LE-ECDSA format.
    InvalidEnvelopeMagic = 3,
    /// Envelope was shorter than the minimum size (71 bytes).
    TruncatedEnvelope = 4,
    /// Envelope's declared payload length did not match the remaining bytes.
    InvalidEnvelopeLength = 5,
    /// Recovered signer pubkey is not in the on-chain trusted set.
    SignerNotTrusted = 6,
    /// Recovered signer is trusted but its expiry has passed.
    SignerExpired = 7,
    /// Envelope's recovery_id byte was outside the valid range `[0, 3]`.
    InvalidRecoveryId = 13,
    /// Verified payload's magic prefix did not match.
    InvalidPayloadMagic = 100,
    /// Verified payload had trailing bytes after decoding finished.
    InvalidPayloadLength = 101,
    /// Verified payload ended mid-field during decoding.
    TruncatedPayload = 102,
    /// Verified payload had an unknown channel value.
    InvalidChannel = 103,
    /// Verified payload had an unknown feed property id.
    InvalidProperty = 104,
    /// Verified payload had an unknown market session value.
    InvalidMarketSession = 105,
    /// Verifier call trapped, or returned an error code this SDK does not
    /// recognize. This is the fallback for non-typed host aborts (e.g. an
    /// invalid signature triggers `secp256k1_recover` to trap rather than
    /// returning a typed error).
    InvokeFailed = 200,
}

impl From<ParseError> for VerifyError {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::TruncatedData => VerifyError::TruncatedPayload,
            ParseError::InvalidPayloadLength => VerifyError::InvalidPayloadLength,
            ParseError::InvalidPayloadMagic => VerifyError::InvalidPayloadMagic,
            ParseError::InvalidChannel => VerifyError::InvalidChannel,
            ParseError::InvalidProperty => VerifyError::InvalidProperty,
            ParseError::InvalidMarketSession => VerifyError::InvalidMarketSession,
        }
    }
}
