#![allow(unused)]
#![allow(unused_imports)]
#![allow(unused_import_braces)]

use bdk_chain::DescriptorExt;
use bdk_chain::DescriptorId;
use bdk_wallet::rusqlite;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use miniscript::{Descriptor, DescriptorPublicKey};

use multi_keychain_wallet::bdk_chain;
use multi_keychain_wallet::multi_keychain::KeyRing;
use multi_keychain_wallet::multi_keychain::Wallet;

// This example shows how to create a BDK wallet from a `KeyRing`.

fn main() -> anyhow::Result<()> {
    let path = ".bdk_example_keyring.sqlite";
    let mut conn = rusqlite::Connection::open(path)?;

    let network = Network::Signet;

    let desc1 = "wpkh([83737d5e/84'/1'/1']tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/0/*)";
    let desc2 = "tr([83737d5e/86'/1'/1']tpubDDR5GgtoxS8fNuSTJU6huqQKGzWshPaemb3UwFDoAXCsyakcQoRcFDMiGUVRX43Lofd7ZB82RcUvu1xnZ5oGZhbr43dRkY8xm2KGhpcq93o/1/*)";

    let desc1_id = get_descriptor_id(desc1);
    let desc2_id = get_descriptor_id(desc2);

    let mut wallet = match Wallet::from_sqlite(&mut conn)? {
        Some(w) => w,
        None => {
            // Create a keyring with an initial and default keychain
            let mut keyring = KeyRing::new(network, desc1_id, desc1);

            // Add a secondary keychain to the keyring
            keyring.add_descriptor(desc2_id, desc2, false);

            let mut wallet = Wallet::new(keyring);
            wallet.persist_to_sqlite(&mut conn)?;
            wallet
        }
    };

    // Reveal an address on the default keychain
    let (indexed, addr) = wallet.reveal_next_default_address_unwrap();
    println!("Address on default keychain:   {:?} {}", indexed, addr);

    // Reveal an address on another keychain
    let (indexed2, addr2) = wallet.reveal_next_address(desc2_id).unwrap();
    println!("Address on secondary keychain: {:?} {}", indexed2, addr2);

    let changeset = wallet.persist_to_sqlite(&mut conn)?;
    println!("Change persisted: {}", changeset.is_some());

    Ok(())
}

/// Helper to pull the descriptor ID out of a descriptor string
fn get_descriptor_id(s: &str) -> DescriptorId {
    let desc = Descriptor::parse_descriptor(&Secp256k1::new(), s)
        .expect("failed to parse descriptor")
        .0;
    desc.descriptor_id()
}
