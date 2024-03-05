use crate::cli_args::EncointerArgsExtractor;
use crate::utils::keys::{KEYSTORE_PATH, SR25519};
use clap::ArgMatches;
use log::info;
use sp_application_crypto::Ss58Codec;
use sp_application_crypto::{ed25519, sr25519};
use sp_keystore::Keystore;
use std::path::PathBuf;
use substrate_client_keystore::KeystoreExt;
use substrate_client_keystore::LocalKeystore;

pub fn new_account(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None).unwrap();

	// This does not place the key into the keystore if we have a seed, but it does
	// place it into the keystore if the seed is none.
	let key = store.sr25519_generate_new(SR25519, matches.seed_arg()).unwrap();

	if let Some(suri) = matches.seed_arg() {
		store.insert(SR25519, suri, &key.0).unwrap();
	}

	drop(store);
	println!("{}", key.to_ss58check());
	Ok(())
}

pub fn list_accounts(_args: &str, _matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None).unwrap();
	info!("sr25519 keys:");
	for pubkey in store.public_keys::<sr25519::AppPublic>().unwrap().into_iter() {
		println!("{}", pubkey.to_ss58check());
	}
	info!("ed25519 keys:");
	for pubkey in store.public_keys::<ed25519::AppPublic>().unwrap().into_iter() {
		println!("{}", pubkey.to_ss58check());
	}
	drop(store);
	Ok(())
}
