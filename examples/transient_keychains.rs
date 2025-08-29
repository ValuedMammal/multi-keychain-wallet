use bitcoin::{Amount, Network};
use multi_keychain_wallet::multi_keychain::KeyRing;
use multi_keychain_wallet::multi_keychain::{fund_keychain, Wallet};

// We can add auxiliary descriptors to our wallet and query their balance separately.

fn main() -> anyhow::Result<()> {
    let desc1 = "tr(tpubDDr5nR9j92Ud2yS68JmUTX2pGPxs6fVXxoxAnisvdWSy9QsYypCdzGbTtqqQtjTGG9WmFpDfvKKJNbUeKnKCX2YA9KpP558QBzqFvTa7C9S/1/*)";
    let desc2 = "tr(tpubDDr5nR9j92Ud2yS68JmUTX2pGPxs6fVXxoxAnisvdWSy9QsYypCdzGbTtqqQtjTGG9WmFpDfvKKJNbUeKnKCX2YA9KpP558QBzqFvTa7C9S/2/*)";
    let desc3 = "tr(tpubDDr5nR9j92Ud2yS68JmUTX2pGPxs6fVXxoxAnisvdWSy9QsYypCdzGbTtqqQtjTGG9WmFpDfvKKJNbUeKnKCX2YA9KpP558QBzqFvTa7C9S/3/*)";
    let desc4 = "tr(tpubDDr5nR9j92Ud2yS68JmUTX2pGPxs6fVXxoxAnisvdWSy9QsYypCdzGbTtqqQtjTGG9WmFpDfvKKJNbUeKnKCX2YA9KpP558QBzqFvTa7C9S/4/*)";

    let keychain1 = KeychainType::KeychainId(1);

    let keychain2 = KeychainType::KeychainId(2);

    let transient1 = KeychainType::TransientKeychain(1);

    let transient2 = KeychainType::TransientKeychain(2);

    let network = Network::Regtest;

    // Create the wallet with our keyring
    let mut keyring = KeyRing::new(network);

    for (keychain_identifier, desc) in [
        (keychain1.clone(), desc1),
        (keychain2.clone(), desc2),
        (transient1.clone(), desc3),
        (transient2.clone(), desc4),
    ] {
        keyring.add_descriptor(keychain_identifier, desc);
    }

    let mut wallet = Wallet::new(keyring);

    // fund the main descriptors
    fund_keychain(&mut wallet, keychain1.clone());

    // Query the balances
    let main_balance: Amount = [keychain1, keychain2]
        .iter()
        .map(|keychain| wallet.keychain_balance(keychain.clone()).total())
        .sum();

    let transient_balance: Amount = [transient1, transient2]
        .iter()
        .map(|keychain| wallet.keychain_balance(keychain.clone()).total())
        .sum();

    println!(
        "Total balance for the main descriptors is: {} sats",
        main_balance.to_sat()
    );
    println!(
        "Total balance for the transient descriptors is: {} sats",
        transient_balance.to_sat()
    );

    Ok(())
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum KeychainType {
    KeychainId(u32),
    TransientKeychain(u32),
}
