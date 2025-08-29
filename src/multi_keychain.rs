//! Module containing the multi-keychain [`Wallet`].

mod changeset;
pub mod keyring;
mod test_util;
mod wallet;

pub use changeset::*;
pub use keyring::KeyRing;
pub use test_util::*;
pub use wallet::*;

/// Alias for [`DescriptorId`](bdk_chain::DescriptorId).
pub(crate) type Did = crate::bdk_chain::DescriptorId;
