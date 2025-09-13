//! `multi_keychain_wallet`

#![warn(missing_docs)]
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

pub use bdk_wallet::chain as bdk_chain;

pub(crate) use bdk_chain::collections;

pub mod multi_keychain;
