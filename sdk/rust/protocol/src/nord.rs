//! Contains all nord modifications for ease of rebase

use crate::api::InvalidFeedSubscriptionDetails;

impl InvalidFeedSubscriptionDetails {
    /// NOTE: seens this empty, wtf duoro labs?
    pub fn is_empty(&self) -> bool {
        self.unknown_ids.is_empty()
            && self.unknown_symbols.is_empty()
            && self.unsupported_channels.is_empty()
            && self.unstable.is_empty()
            && self.not_entitled.is_empty()
    }
}