use bdk_wallet::KeychainKind;
use bitcoin::Network;
use multi_keychain_wallet::multi_keychain::{KeyRing, Wallet};

use bdk_electrum::{electrum_client::Client, BdkElectrumClient};

const STOP_GAP: usize = 50;
const BATCH_SIZE: usize = 5;
const EXTERNAL_DESCRIPTOR: &str = "tr(tprv8ZgxMBicQKsPdrjwWCyXqqJ4YqcyG4DmKtjjsRt29v1PtD3r3PuFJAjWytzcvSTKnZAGAkPSmnrdnuHWxCAwy3i1iPhrtKAfXRH7dVCNGp6/86'/1'/0'/0/*)#g9xn7wf9";
const INTERNAL_DESCRIPTOR: &str = "tr(tprv8ZgxMBicQKsPdrjwWCyXqqJ4YqcyG4DmKtjjsRt29v1PtD3r3PuFJAjWytzcvSTKnZAGAkPSmnrdnuHWxCAwy3i1iPhrtKAfXRH7dVCNGp6/86'/1'/0'/1/*)#e3rjrmea";

fn main() {
    // Construct wallet
    let mut keyring =
        KeyRing::<KeychainKind>::new(Network::Signet, KeychainKind::External, EXTERNAL_DESCRIPTOR);
    keyring.add_descriptor(KeychainKind::Internal, INTERNAL_DESCRIPTOR, false);

    let mut wallet = Wallet::new(keyring);

    let balance = wallet.balance();
    println!("Balance before syncing: {} sats", balance.total().to_sat());

    // Reveal address
    let address = wallet.reveal_next_default_address_unwrap();
    println!("Address revealed: {}", address.address);

    let client = BdkElectrumClient::new(Client::new("ssl://mempool.space:60602").unwrap());

    // Perform sync
    let sync_request = wallet.start_sync_with_revealed_spks();
    let update = client.sync(sync_request, BATCH_SIZE, true).unwrap();

    wallet.apply_update(update);

    let balance = wallet.balance();
    println!("Balance after sync: {} sats", balance.total().to_sat());

    // Perform full scan
    let full_scan_request = wallet.start_full_scan();
    let update = client
        .full_scan(full_scan_request, STOP_GAP, BATCH_SIZE, true)
        .unwrap();

    wallet.apply_update(update);

    let balance = wallet.balance();
    println!("Balance after full scan: {} sats", balance.total().to_sat());
}
