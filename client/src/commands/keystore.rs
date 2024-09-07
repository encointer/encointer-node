use crate::{
	cli_args::EncointerArgsExtractor,
	utils::keys::{KEYSTORE_PATH, SR25519},
};
use ac_keystore::{KeystoreExt, LocalKeystore};
use clap::ArgMatches;
use log::info;
use sp_application_crypto::{ed25519, sr25519, Ss58Codec};
use sp_keystore::Keystore;
use std::{env, fs, io::Read, path::PathBuf};

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

pub fn export_secret(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let arg_account = matches.value_of("account").unwrap();
	let mut path = env::current_dir().expect("Failed to get current directory");
	path.push("my_keystore");
	let pubkey = sr25519::Public::from_ss58check(arg_account)
		.expect("arg should be ss58 encoded public key");
	let key_type = array_bytes::bytes2hex("", SR25519.0);
	let key = array_bytes::bytes2hex("", pubkey);
	path.push(key_type + key.as_str());
	let mut file = fs::File::open(&path).expect("Failed to open keystore file");
	let mut contents = String::new();
	file.read_to_string(&mut contents).expect("Failed to read file contents");
	println!("{}", contents);
	Ok(())
}
