use crate::bdk_chain::{BlockId, ConfirmationBlockTime, TxUpdate};
use crate::multi_keychain::{Update, Wallet};
use alloc::sync::Arc;
use bitcoin::{
    absolute, hashes::Hash, transaction, Address, Amount, BlockHash, Network, OutPoint,
    Transaction, TxIn, TxOut, Txid,
};
use core::{fmt, str::FromStr};

pub fn fund_keychain<K>(wallet: &mut Wallet<K>, keychain: K)
where
    K: fmt::Debug + Clone + Ord,
{
    let receive_address = wallet.reveal_next_address(keychain).unwrap().1;
    let sendto_address = Address::from_str("bcrt1q395f053hx4rc90n37a8h6ke8q6hv4rdemyewjv")
        .expect("address")
        .require_network(Network::Regtest)
        .unwrap();

    let tx0 = Transaction {
        output: vec![TxOut {
            value: Amount::from_sat(76_000),
            script_pubkey: receive_address.script_pubkey(),
        }],
        ..new_tx(0)
    };

    let tx1 = Transaction {
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: tx0.compute_txid(),
                vout: 0,
            },
            ..Default::default()
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(50_000),
                script_pubkey: receive_address.script_pubkey(),
            },
            TxOut {
                value: Amount::from_sat(25_000),
                script_pubkey: sendto_address.script_pubkey(),
            },
        ],
        ..new_tx(0)
    };

    insert_checkpoint(
        wallet,
        BlockId {
            height: 42,
            hash: BlockHash::all_zeros(),
        },
    );
    insert_checkpoint(
        wallet,
        BlockId {
            height: 1_000,
            hash: BlockHash::all_zeros(),
        },
    );
    insert_checkpoint(
        wallet,
        BlockId {
            height: 2_000,
            hash: BlockHash::all_zeros(),
        },
    );

    insert_tx(wallet, tx0.clone());
    insert_anchor(
        wallet,
        tx0.compute_txid(),
        ConfirmationBlockTime {
            block_id: BlockId {
                height: 1_000,
                hash: BlockHash::all_zeros(),
            },
            confirmation_time: 100,
        },
    );

    insert_tx(wallet, tx1.clone());
    insert_anchor(
        wallet,
        tx1.compute_txid(),
        ConfirmationBlockTime {
            block_id: BlockId {
                height: 2_000,
                hash: BlockHash::all_zeros(),
            },
            confirmation_time: 200,
        },
    );
}

pub fn insert_checkpoint<K>(wallet: &mut Wallet<K>, block: BlockId)
where
    K: fmt::Debug + Clone + Ord,
{
    let mut cp = wallet.latest_checkpoint();
    cp = cp.insert(block);
    wallet.apply_update(Update {
        chain: Some(cp),
        ..Default::default()
    });
}

/// Inserts a transaction into the local view, assuming it is currently present in the mempool.
///
/// This can be used, for example, to track a transaction immediately after it is broadcast.
pub fn insert_tx<K>(wallet: &mut Wallet<K>, tx: Transaction)
where
    K: fmt::Debug + Clone + Ord,
{
    let txid = tx.compute_txid();
    let seen_at = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs();
    let mut tx_update = TxUpdate::default();
    tx_update.txs = vec![Arc::new(tx)];
    tx_update.seen_ats = [(txid, seen_at)].into();
    wallet.apply_update(Update {
        tx_update,
        ..Default::default()
    });
}

/// Simulates confirming a tx with `txid` by applying an update to the wallet containing
/// the given `anchor`. Note: to be considered confirmed the anchor block must exist in
/// the current active chain.
pub fn insert_anchor<K>(wallet: &mut Wallet<K>, txid: Txid, anchor: ConfirmationBlockTime)
where
    K: fmt::Debug + Clone + Ord,
{
    let mut tx_update = TxUpdate::default();
    tx_update.anchors = [(anchor, txid)].into();
    wallet.apply_update(Update {
        tx_update,
        ..Default::default()
    });
}

/// A new empty transaction with the given locktime
pub fn new_tx(locktime: u32) -> Transaction {
    Transaction {
        version: transaction::Version::ONE,
        lock_time: absolute::LockTime::from_consensus(locktime),
        input: vec![],
        output: vec![],
    }
}

#[cfg(test)]
mod test {
    use crate::multi_keychain::KeyRing;

    use super::*;

    #[test]
    fn keychain_is_funded() {
        use crate::bdk_chain::DescriptorExt;
        use bitcoin::secp256k1::Secp256k1;
        use miniscript::Descriptor;
        let desc_str = "tr(tpubDDr5nR9j92Ud2yS68JmUTX2pGPxs6fVXxoxAnisvdWSy9QsYypCdzGbTtqqQtjTGG9WmFpDfvKKJNbUeKnKCX2YA9KpP558QBzqFvTa7C9S/1/*)";
        let desc = Descriptor::parse_descriptor(&Secp256k1::new(), desc_str)
            .expect("failed to parse descriptor")
            .0;
        let did = desc.descriptor_id();
        let mut keyring = KeyRing::new(Network::Regtest);
        keyring.add_descriptor(did, desc);
        let mut wallet = Wallet::new(keyring);
        fund_keychain(&mut wallet, did);
        assert_eq!(wallet.total_balance().confirmed, Amount::from_sat(50_000));
    }
}
