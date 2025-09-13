use core::{fmt, ops::Deref};

use bitcoin::Address;
use miniscript::{Descriptor, DescriptorPublicKey};

#[cfg(feature = "rusqlite")]
use bdk_chain::rusqlite;
use bdk_chain::{
    keychain_txout::{
        FullScanRequestBuilderExt, KeychainTxOutIndex, SyncRequestBuilderExt, DEFAULT_LOOKAHEAD,
    },
    local_chain::LocalChain,
    spk_client::{
        FullScanRequest, FullScanRequestBuilder, FullScanResponse, SyncRequest, SyncRequestBuilder,
        SyncResponse,
    },
    CheckPoint, ConfirmationBlockTime, IndexedTxGraph, Merge,
};

use crate::bdk_chain;
use crate::collections::BTreeMap;
use crate::multi_keychain::{ChangeSet, KeyRing};

/// Alias for a [`IndexedTxGraph`].
type KeychainTxGraph<K> = IndexedTxGraph<ConfirmationBlockTime, KeychainTxOutIndex<K>>;

// This is here for dev purposes and can be made a configurable option as part of the final API.
const USE_SPK_CACHE: bool = false;

/// [`Wallet`] is a structure that stores transaction data that can be indexed by multiple
/// keychains.
#[derive(Debug)]
pub struct Wallet<K: Ord> {
    keyring: KeyRing<K>,
    chain: LocalChain,
    tx_graph: KeychainTxGraph<K>,
    stage: ChangeSet<K>,
}

impl<K> Wallet<K>
where
    K: fmt::Debug + Clone + Ord,
{
    /// Construct a new [`Wallet`] with the given `keyring`.
    pub fn new(mut keyring: KeyRing<K>) -> Self {
        let network = keyring.network;

        let genesis_hash = bitcoin::constants::genesis_block(network).block_hash();
        let (chain, chain_changeset) = LocalChain::from_genesis_hash(genesis_hash);

        let keyring_changeset = keyring.initial_changeset();

        let mut index = KeychainTxOutIndex::new(DEFAULT_LOOKAHEAD, USE_SPK_CACHE);
        let descriptors = core::mem::take(&mut keyring.descriptors);
        for (keychain, desc) in descriptors {
            let _inserted = index
                .insert_descriptor(keychain, desc)
                .expect("err: failed to insert descriptor");
            assert!(_inserted);
        }

        let tx_graph = KeychainTxGraph::new(index);

        let stage = ChangeSet {
            keyring: keyring_changeset,
            local_chain: chain_changeset,
            tx_graph: bdk_chain::tx_graph::ChangeSet::default(),
            indexer: bdk_chain::keychain_txout::ChangeSet::default(),
        };

        Self {
            keyring,
            chain,
            tx_graph,
            stage,
        }
    }

    /// Construct [`Wallet`] from the provided `changeset`.
    ///
    /// Will be `None` if the changeset is empty.
    pub fn from_changeset(changeset: ChangeSet<K>) -> Option<Self> {
        if changeset.is_empty() {
            return None;
        }

        // chain
        let chain =
            LocalChain::from_changeset(changeset.local_chain).expect("err: Missing genesis");

        // keyring
        let mut keyring = KeyRing::from_changeset(changeset.keyring)?;

        // index
        let mut index = KeychainTxOutIndex::new(DEFAULT_LOOKAHEAD, USE_SPK_CACHE);
        index.apply_changeset(changeset.indexer);
        for (keychain, descriptor) in core::mem::take(&mut keyring.descriptors) {
            let _inserted = index
                .insert_descriptor(keychain, descriptor)
                .expect("failed to insert descriptor");
            assert!(_inserted);
        }

        // txgraph
        let mut tx_graph = KeychainTxGraph::new(index);
        tx_graph.apply_changeset(changeset.tx_graph.into());

        let stage = ChangeSet::default();

        Some(Self {
            tx_graph,
            stage,
            chain,
            keyring,
        })
    }

    /// Reveal next default address. Panics if the default implementation of `K` does not match
    /// a keychain contained in this wallet.
    pub fn reveal_next_default_address_unwrap(&mut self) -> AddressInfo<K> {
        self.reveal_next_address(self.keyring.default_keychain())
            .expect("invalid keychain")
    }

    /// Reveal next address from the given `keychain`.
    ///
    /// This may return the last revealed address in case there are none left to reveal.
    pub fn reveal_next_address(&mut self, keychain: K) -> Option<AddressInfo<K>> {
        let ((index, spk), index_changeset) =
            self.tx_graph.index.reveal_next_spk(keychain.clone())?;
        let address = Address::from_script(&spk, self.keyring.network)
            .expect("script should have address form");

        self.stage(index_changeset);

        Some(AddressInfo {
            index,
            address,
            keychain,
        })
    }

    /// Iterate over `(keychain, descriptor)` pairs contained in this wallet.
    pub fn keychains(
        &self,
    ) -> impl DoubleEndedIterator<Item = (K, &Descriptor<DescriptorPublicKey>)> {
        self.tx_graph.index.keychains()
    }

    /// Get the default keychain
    pub fn default_keychain(&self) -> K {
        self.keyring.default_keychain()
    }

    /// Compute the balance.
    pub fn balance(&self) -> bdk_chain::Balance {
        use bdk_chain::CanonicalizationParams;
        let chain = &self.chain;
        let outpoints = self.tx_graph.index.outpoints().clone();
        self.tx_graph.graph().balance(
            chain,
            chain.tip().block_id(),
            CanonicalizationParams::default(),
            outpoints,
            |_, _| false,
        )
    }

    /// Obtain a reference to the indexed transaction graph.
    pub fn tx_graph(&self) -> &IndexedTxGraph<ConfirmationBlockTime, KeychainTxOutIndex<K>> {
        &self.tx_graph
    }

    /// Obtain a reference to the txout index.
    pub fn txout_index(&self) -> &KeychainTxOutIndex<K> {
        &self.tx_graph.index
    }

    /// Obtain a reference to the local chain.
    pub fn local_chain(&self) -> &LocalChain {
        &self.chain
    }

    /// Apply update.
    pub fn apply_update(&mut self, update: impl Into<Update<K>>) {
        let Update {
            chain,
            tx_update,
            last_active_indices,
        } = update.into();

        let mut changeset = ChangeSet::default();

        // chain
        if let Some(tip) = chain {
            changeset.merge(
                self.chain
                    .apply_update(tip)
                    .expect("err: failed to apply update to chain")
                    .into(),
            );
        }
        // index
        changeset.merge(
            self.tx_graph
                .index
                .reveal_to_target_multi(&last_active_indices)
                .into(),
        );
        // tx graph
        changeset.merge(self.tx_graph.apply_update(tx_update).into());

        self.stage(changeset);
    }

    /// Stages anything that can be converted directly into a [`ChangeSet`].
    fn stage(&mut self, changeset: impl Into<ChangeSet<K>>) {
        self.stage.merge(changeset.into());
    }

    /// See the staged changes if any.
    pub fn staged(&self) -> Option<&ChangeSet<K>> {
        if self.stage.is_empty() {
            None
        } else {
            Some(&self.stage)
        }
    }
}

#[cfg(feature = "rusqlite")]
use bdk_chain::DescriptorId;

// TODO: This should probably be handled by `PersistedWallet` or similar
#[cfg(feature = "rusqlite")]
impl Wallet<DescriptorId> {
    /// Construct [`Wallet`] from SQLite.
    pub fn from_sqlite(conn: &mut rusqlite::Connection) -> rusqlite::Result<Option<Self>> {
        let tx = conn.transaction()?;

        let changeset = ChangeSet::initialize(&tx)?;
        tx.commit()?;

        Ok(changeset.and_then(Self::from_changeset))
    }

    /// Persist to SQLite. Returns the newly committed changeset if successful, or `None`
    /// if the stage is currently empty.
    pub fn persist_to_sqlite(
        &mut self,
        conn: &mut rusqlite::Connection,
    ) -> rusqlite::Result<Option<ChangeSet<DescriptorId>>> {
        let mut ret = None;

        let tx = conn.transaction()?;

        if let Some(changeset) = self.staged_changeset() {
            changeset.persist_to_sqlite(&tx)?;
            tx.commit()?;
            ret = self.stage.take();
        }

        Ok(ret)
    }

    /// See the staged changes if any.
    pub fn staged_changeset(&self) -> Option<&ChangeSet<DescriptorId>> {
        if self.stage.is_empty() {
            None
        } else {
            Some(&self.stage)
        }
    }
}

/// A derived address and the index it was found at.
/// For convenience this automatically derefs to `Address`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressInfo<K> {
    /// Child index of this address
    pub index: u32,
    /// Address
    pub address: Address,
    /// Type of keychain
    pub keychain: K,
}

impl<K> Deref for AddressInfo<K> {
    type Target = Address;

    fn deref(&self) -> &Self::Target {
        &self.address
    }
}

impl<K> fmt::Display for AddressInfo<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.address)
    }
}

/// Contains structures for updating a multi-keychain wallet.
#[derive(Debug)]
pub struct Update<K> {
    /// chain
    pub chain: Option<CheckPoint>,
    /// tx update
    pub tx_update: bdk_chain::TxUpdate<ConfirmationBlockTime>,
    /// last active keychain indices
    pub last_active_indices: BTreeMap<K, u32>,
}

impl<K> From<FullScanResponse<K>> for Update<K> {
    fn from(resp: bdk_chain::spk_client::FullScanResponse<K>) -> Self {
        Self {
            chain: resp.chain_update,
            tx_update: resp.tx_update,
            last_active_indices: resp.last_active_indices,
        }
    }
}

impl<K> From<SyncResponse> for Update<K> {
    fn from(resp: bdk_chain::spk_client::SyncResponse) -> Self {
        Self {
            chain: resp.chain_update,
            tx_update: resp.tx_update,
            last_active_indices: BTreeMap::new(),
        }
    }
}

/// Methods to construct sync/full-scan requests for spk-based chain sources.
impl<K> Wallet<K>
where
    K: Ord + Clone + fmt::Debug,
{
    /// Create a partial [`SyncRequest`] for all revealed spks at `start_time`.
    pub fn start_sync_with_revealed_spks_at(
        &self,
        start_time: u64,
    ) -> SyncRequestBuilder<(K, u32)> {
        SyncRequest::builder_at(start_time)
            .chain_tip(self.chain.tip())
            .revealed_spks_from_indexer(&self.tx_graph.index, ..)
            .expected_spk_txids(self.tx_graph.list_expected_spk_txids(
                &self.chain,
                self.chain.tip().block_id(),
                ..,
            ))
    }

    /// Create a partial [`SyncRequest`] for all revealed spks at the current system time.
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[cfg(feature = "std")]
    pub fn start_sync_with_revealed_spks(&self) -> SyncRequestBuilder<(K, u32)> {
        SyncRequest::builder()
            .chain_tip(self.chain.tip())
            .revealed_spks_from_indexer(&self.tx_graph.index, ..)
            .expected_spk_txids(self.tx_graph.list_expected_spk_txids(
                &self.chain,
                self.chain.tip().block_id(),
                ..,
            ))
    }

    /// Create a [`FullScanRequest`] at the `start_time` time.
    pub fn start_full_scan_at(&self, start_time: u64) -> FullScanRequestBuilder<K> {
        FullScanRequest::builder_at(start_time)
            .chain_tip(self.chain.tip())
            .spks_from_indexer(&self.tx_graph.index)
    }

    /// Create a [`FullScanRequest`] at the current system time.
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[cfg(feature = "std")]
    pub fn start_full_scan(&self) -> FullScanRequestBuilder<K> {
        FullScanRequest::builder()
            .chain_tip(self.chain.tip())
            .spks_from_indexer(&self.tx_graph.index)
    }
}

#[cfg(test)]
mod test {
    use crate::bdk_chain::{DescriptorExt, DescriptorId};
    use crate::multi_keychain::{KeyRing, Wallet};
    use bitcoin::{secp256k1::Secp256k1, Network};
    use miniscript::Descriptor;
    use tempfile::NamedTempFile;

    #[cfg(feature = "rusqlite")]
    use crate::bdk_chain::rusqlite;

    const DESCRIPTORS: [&str; 6] = ["wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/0/*)", "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/1/*)", "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/2/*)", "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/3/*)", "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/4/*)", "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/5/*)"];

    fn descriptor_id(s: &str) -> DescriptorId {
        let desc = Descriptor::parse_descriptor(&Secp256k1::new(), s)
            .expect("failed to parse descriptor")
            .0;
        desc.descriptor_id()
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn persist_default() -> anyhow::Result<()> {
        let db_file = NamedTempFile::new()?;
        let mut conn = rusqlite::Connection::open(db_file.path())?;
        let desc_id = descriptor_id(DESCRIPTORS[0]);
        let keyring = KeyRing::new(Network::Signet, desc_id, DESCRIPTORS[0]);

        {
            let _ = Wallet::<DescriptorId>::from_sqlite(&mut conn)?;
            let mut wallet = Wallet::<DescriptorId>::new(keyring);
            wallet.persist_to_sqlite(&mut conn)?;
        }

        {
            let wallet = Wallet::from_sqlite(&mut conn)?.unwrap();
            assert_eq!(wallet.default_keychain(), desc_id);
        }

        Ok(())
    }
}
