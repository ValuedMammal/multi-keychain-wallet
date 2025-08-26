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
    let path = ".example_default_keychain.sqlite3";
    let mut conn = rusqlite::Connection::open(path)?;

    let network = Network::Signet;

    let desc1 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/0/*)";
    let desc2 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/1/*)";
    let desc3 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/2/*)";
    let desc4 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/3/*)";
    let desc5 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/4/*)";
    let desc6 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/5/*)";

    let mut wallet = match Wallet::from_sqlite(&mut conn)? {
        Some(w) => w,
        None => {
            // Descriptor 1 will be used as the default keychain on this wallet
            let desc_id = get_descriptor_id(desc1);
            let mut keyring = KeyRing::new(network, desc_id, desc1);

            // We then add descriptors to this keyring
            for descriptor_string in [desc2, desc3, desc4, desc5, desc6, desc6] {
                let desc_id = get_descriptor_id(descriptor_string);
                keyring.add_descriptor(desc_id, descriptor_string, false);
            }

            // Creating a wallet requires a single argument! A valid keyring.
            let mut wallet = Wallet::new(keyring);
            wallet.persist_to_sqlite(&mut conn)?;
            wallet
        }
    };

    let (indexed, addr) = wallet.reveal_next_default_address_unwrap();
    println!("Address: {:?} {}", indexed, addr);

    let changeset = wallet.persist_to_sqlite(&mut conn)?;
    println!("Change persisted: {}", changeset.is_some());

    Ok(())
}

/// Helper to match descriptors with their descriptor ID.
fn get_descriptor_id(s: &str) -> DescriptorId {
    let desc = Descriptor::parse_descriptor(&Secp256k1::new(), s)
        .expect("failed to parse descriptor")
        .0;
    desc.descriptor_id()
}
