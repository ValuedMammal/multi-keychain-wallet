#![allow(unused)]
#![allow(unused_imports)]
#![allow(unused_import_braces)]

use bdk_chain::DescriptorExt;
use bdk_chain::DescriptorId;
use bdk_wallet::rusqlite;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use miniscript::{Descriptor, DescriptorPublicKey};
use miniscript::descriptor::DescriptorType;
use multi_keychain_wallet::bdk_chain;
use multi_keychain_wallet::multi_keychain::KeyRing;
use multi_keychain_wallet::multi_keychain::Wallet;

// The KeyRing holds a map of keychain identifiers (`K`) to public descriptors. These keychain identifiers can be simple
// (something like the `DescriptorId` type works well), but it can also be more complex if required by the application.
// This example shows how the keychain identifier can be used to carry metadata about the descriptors, which could be used
// to select which keychain to use in different scenarios when calling methods like `Wallet::reveal_next_address`.

fn main() -> anyhow::Result<()> {
    let desc1 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/1/*)";
    let desc2 = "wpkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/2/*)";
    let desc3 = "tr(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/3/*)";
    let desc4 = "tr(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/4/*)";
    let desc5 = "pkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/5/*)";
    let desc6 = "pkh(tpubDCzuCBKnZA5TNKhiJnASku7kq8Q4iqcVF82JV7mHo2NxWpXkLRbrJaGA5ToE7LCuWpcPErBbpDzbdWKN8aTdJzmRy1jQPmZvnqpwwDwCdy7/6/*)";

    let keychain_johnny = KeychainId {
        number: 1,
        nickname: "Johnny's keychain".to_string(),
        script_type: DescriptorType::Wpkh,
        color: Color::Blue
    };
    let keychain_samantha = KeychainId {
        number: 2,
        nickname: "Samantha's keychain".to_string(),
        script_type: DescriptorType::Wpkh,
        color: Color::Green
    };
    let keychain_riley = KeychainId {
        number: 3,
        nickname: "Riley's keychain".to_string(),
        script_type: DescriptorType::Tr,
        color: Color::Yellow
    };
    let keychain_max = KeychainId {
        number: 4,
        nickname: "Max's keychain".to_string(),
        script_type: DescriptorType::Tr,
        color: Color::Blue
    };
    let keychain_penelope = KeychainId {
        number: 5,
        nickname: "Penelope's keychain".to_string(),
        script_type: DescriptorType::Pkh,
        color: Color::Green
    };
    let keychain_george = KeychainId {
        number: 6,
        nickname: "George's keychain".to_string(),
        script_type: DescriptorType::Pkh,
        color: Color::Yellow
    };

    let network = Network::Signet;

    // Create a new keyring with our custom KeychainId type
    let mut keyring = KeyRing::new(network);

    // Assign descriptors to keychains
    for (keychain_identifier, desc) in [
        (keychain_johnny.clone(), desc1),
        (keychain_samantha.clone(), desc2),
        (keychain_riley.clone(), desc3),
        (keychain_max.clone(), desc4),
        (keychain_penelope.clone(), desc5),
        (keychain_george.clone(), desc6),
    ] {
        keyring.add_descriptor(keychain_identifier, desc);
    }

    // Create a new wallet with our keyring
    let mut wallet = Wallet::new(keyring);

    // Reveal addresses for each keychain
    println!("\nRevealing addresses for each keychain\n{}", "=".repeat(50));

    let (keychain_and_index, addr) = wallet.reveal_next_address(keychain_johnny.clone()).unwrap();
    println!("Johnny's address (index {:?}):   {}", keychain_and_index.1, addr);

    let (keychain_and_index, addr) = wallet.reveal_next_address(keychain_samantha.clone()).unwrap();
    println!("Samantha's address (index {:?}): {}", keychain_and_index.1, addr);

    let (keychain_and_index, addr) = wallet.reveal_next_address(keychain_riley.clone()).unwrap();
    println!("Riley's address (index {:?}):    {}", keychain_and_index.1, addr);

    let (keychain_and_index, addr) = wallet.reveal_next_address(keychain_max.clone()).unwrap();
    println!("Max's address (index {:?}):      {}", keychain_and_index.1, addr);

    let (keychain_and_index, addr) = wallet.reveal_next_address(keychain_penelope.clone()).unwrap();
    println!("Penelope's address (index {:?}): {}", keychain_and_index.1, addr);

    let (keychain_and_index, addr) = wallet.reveal_next_address(keychain_george.clone()).unwrap();
    println!("George's address (index {:?}):   {}", keychain_and_index.1, addr);

    Ok(())
}

#[derive(Debug, Clone)]
struct KeychainId {
    number: u32,
    nickname: String,
    script_type: DescriptorType,
    color: Color,
}

impl PartialEq for KeychainId {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

impl Eq for KeychainId {}

impl PartialOrd for KeychainId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KeychainId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.number.cmp(&other.number)
    }
}

#[derive(Debug, Clone)]
enum Color {
    Blue,
    Green,
    Yellow,
}
