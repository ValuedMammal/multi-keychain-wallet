use miniscript::Descriptor;
use multi_keychain_wallet::bdk_chain::{DescriptorExt, DescriptorId};
use std::sync::Arc;

use bitcoin::{secp256k1::Secp256k1, Network, Transaction};
use multi_keychain_wallet::multi_keychain::{KeyRing, Wallet};

use bdk_bitcoind_rpc::{
    bitcoincore_rpc::{Auth, Client},
    Emitter,
};

const USER: &str = "alice";
const PASSWORD: &str = "password";
const EXTERNAL_DESCRIPTOR: &str = "tr(tprv8ZgxMBicQKsPdrjwWCyXqqJ4YqcyG4DmKtjjsRt29v1PtD3r3PuFJAjWytzcvSTKnZAGAkPSmnrdnuHWxCAwy3i1iPhrtKAfXRH7dVCNGp6/86'/1'/0'/0/*)#g9xn7wf9";
const INTERNAL_DESCRIPTOR: &str = "tr(tprv8ZgxMBicQKsPdrjwWCyXqqJ4YqcyG4DmKtjjsRt29v1PtD3r3PuFJAjWytzcvSTKnZAGAkPSmnrdnuHWxCAwy3i1iPhrtKAfXRH7dVCNGp6/86'/1'/0'/1/*)#e3rjrmea";

fn main() {
    let mut keyring = KeyRing::<DescriptorId>::new(
        Network::Regtest,
        get_descriptor_id(EXTERNAL_DESCRIPTOR),
        EXTERNAL_DESCRIPTOR,
    );
    keyring.add_descriptor(
        get_descriptor_id(INTERNAL_DESCRIPTOR),
        INTERNAL_DESCRIPTOR,
        false,
    );
    let mut wallet = Wallet::new(keyring);

    let balance = wallet.balance();
    println!("Balance before syncing: {}", balance);

    let address = wallet.reveal_next_default_address_unwrap();
    println!("Address revealed: {}", address.address);

    let rpc_client: Client = Client::new(
        "http://127.0.0.1:18443",
        Auth::UserPass(USER.to_string(), PASSWORD.to_string()),
    )
    .unwrap();

    let wallet_tip = wallet.latest_checkpoint();
    println!(
        "Current wallet tip is at hash: {} and height:{}",
        wallet_tip.hash(),
        wallet_tip.height()
    );

    let mut emitter = Emitter::new(
        &rpc_client,
        wallet_tip.clone(),
        wallet_tip.height(),
        std::iter::empty::<Arc<Transaction>>(),
    );

    println!("Syncing blocks...");

    while let Some(block) = emitter.next_block().unwrap() {
        wallet
            .apply_block_connected_to(&block.block, block.block_height(), block.connected_to())
            .unwrap();
    }

    let new_wallet_tip = wallet.latest_checkpoint();
    println!(
        "Current wallet tip is at hash: {} and height:{}",
        new_wallet_tip.hash(),
        new_wallet_tip.height()
    );

    println!("Syncing mempool...");
    let mempool_emissions = emitter.mempool().unwrap();
    wallet.apply_unconfirmed_txs(mempool_emissions.update);
    wallet.apply_evicted_txs(mempool_emissions.evicted);

    let balance = wallet.balance();
    println!("Balance after syncing: {}", balance);
}

/// Helper to pull the descriptor ID out of a descriptor string
fn get_descriptor_id(s: &str) -> DescriptorId {
    let desc = Descriptor::parse_descriptor(&Secp256k1::new(), s)
        .expect("failed to parse descriptor")
        .0;
    desc.descriptor_id()
}
