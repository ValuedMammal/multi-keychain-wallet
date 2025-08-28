//! [`KeyRing`].

use bdk_chain::{DescriptorExt, Merge};
use bdk_wallet::descriptor::IntoWalletDescriptor;
use bitcoin::{
    secp256k1::{All, Secp256k1},
    Network,
};
use miniscript::{Descriptor, DescriptorPublicKey};
use serde::{Deserialize, Serialize};

use crate::bdk_chain;
use crate::collections::BTreeMap;
use crate::multi_keychain::Did;

/// KeyRing.
#[derive(Debug, Clone)]
pub struct KeyRing<K> {
    pub(crate) secp: Secp256k1<All>,
    pub(crate) network: Network,
    pub(crate) descriptors: BTreeMap<K, Descriptor<DescriptorPublicKey>>,
    pub(crate) default_keychain: K,
}

impl<K> KeyRing<K>
where
    K: Ord + Clone,
{
    /// Construct a new [`KeyRing`] with the provided `network` and a tuple containing a keychain
    /// identifier and public descriptor. This keychain is automatically designed as the default
    /// keychain.
    pub fn new(network: Network, default_keychain: (K, Descriptor<DescriptorPublicKey>)) -> Self {
        Self {
            secp: Secp256k1::new(),
            network,
            descriptors: BTreeMap::from([(default_keychain.0.clone(), default_keychain.1)]),
            default_keychain: default_keychain.0.clone(),
        }
    }

    /// Add a descriptor, must not be [multipath](miniscript::Descriptor::is_multipath).
    pub fn add_descriptor(
        &mut self,
        keychain: K,
        descriptor: impl IntoWalletDescriptor,
        default: bool,
    ) {
        let descriptor = descriptor
            .into_wallet_descriptor(&self.secp, self.network)
            .expect("err: invalid descriptor")
            .0;
        assert!(
            !descriptor.is_multipath(),
            "err: Use `add_multipath_descriptor` instead"
        );

        if default || self.descriptors.is_empty() {
            self.default_keychain = keychain.clone();
        }
        self.descriptors.insert(keychain, descriptor);
    }

    /// Returns the specified default keychain on the KeyRing.
    pub fn default_keychain(&self) -> K {
        self.default_keychain.clone()
    }

    /// Initial changeset.
    pub fn initial_changeset(&self) -> ChangeSet<K> {
        ChangeSet {
            network: Some(self.network),
            descriptors: self.descriptors.clone(),
            default_keychain: Some(self.default_keychain.clone()),
        }
    }

    /// Construct from changeset.
    pub fn from_changeset(changeset: ChangeSet<K>) -> Option<Self> {
        Some(Self {
            secp: Secp256k1::new(),
            network: changeset.network?,
            descriptors: changeset.descriptors,
            default_keychain: changeset.default_keychain?,
        })
    }
}

impl KeyRing<Did> {
    /// Create a new [`KeyRing`] from a multipath descriptor. You must specify a default keychain by providing
    /// a default_keychain argument corresponding to one of the descriptors in the multipath.
    pub fn new_multipath(network: Network, descriptor: impl IntoWalletDescriptor, default_keychain: usize) -> Self {
        let descriptor = descriptor
            .into_wallet_descriptor(&Secp256k1::new(), network)
            .expect("err: invalid descriptor")
            .0;
        assert!(
            descriptor.is_multipath(),
            "err: Use `add_descriptor` instead"
        );
        let descriptors = descriptor
            .into_single_descriptors()
            .expect("err: invalid descriptor");

        // The default keychain is the one at index specified by the user
        assert!(descriptors.len() >= default_keychain);
        let default_keychain = descriptors[default_keychain].descriptor_id();

        let descriptors_map = descriptors
            .into_iter()
            .map(|desc| (desc.descriptor_id(), desc))
            .collect();

        Self {
            secp: Secp256k1::new(),
            network,
            descriptors: descriptors_map,
            default_keychain: default_keychain.clone(),
        }
    }

    /// Add multipath descriptor.
    pub fn add_multipath_descriptor(&mut self, descriptor: impl IntoWalletDescriptor) {
        let descriptor = descriptor
            .into_wallet_descriptor(&self.secp, self.network)
            .expect("err: invalid descriptor")
            .0;
        assert!(
            descriptor.is_multipath(),
            "err: Use `add_descriptor` instead"
        );
        let descriptors = descriptor
            .into_single_descriptors()
            .expect("err: invalid descriptor");
        for descriptor in descriptors {
            let did = descriptor.descriptor_id();
            self.descriptors.insert(did, descriptor);
        }
    }
}

/// Represents changes to the `KeyRing`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet<K: Ord> {
    /// Network.
    pub network: Option<Network>,
    /// Added descriptors.
    pub descriptors: BTreeMap<K, Descriptor<DescriptorPublicKey>>,
    /// Default keychain
    pub default_keychain: Option<K>,
}

impl<K: Ord> Default for ChangeSet<K> {
    fn default() -> Self {
        Self {
            network: None,
            descriptors: Default::default(),
            default_keychain: None,
        }
    }
}

impl<K: Ord> Merge for ChangeSet<K> {
    fn merge(&mut self, other: Self) {
        // merge network
        if other.network.is_some() && self.network.is_none() {
            self.network = other.network;
        }
        // merge descriptors
        self.descriptors.extend(other.descriptors);
        // Note: if a new default keychain has been set, it will take precedence over the old one.
        self.default_keychain = other.default_keychain;
    }

    fn is_empty(&self) -> bool {
        self.network.is_none() && self.descriptors.is_empty()
    }
}
