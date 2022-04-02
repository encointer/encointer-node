use codec::Encode;
use encointer_primitives::scheduler::CeremonyIndexType;
use sp_application_crypto::sr25519;
use sp_core::Pair;
use substrate_api_client::{
	compose_call, compose_extrinsic_offline, rpc::WsRpcClient, Api, Metadata, UncheckedExtrinsicV4,
};

/// Wrapper around the `compose_extrinsic_offline!` macro to be less verbose.
pub fn offline_xt<C: Encode + Clone>(
	api: &Api<sr25519::Pair, WsRpcClient>,
	call: C,
	nonce: u32,
) -> UncheckedExtrinsicV4<C> {
	compose_extrinsic_offline!(
		api.clone().signer.unwrap(),
		call,
		nonce,
		Era::Immortal,
		api.genesis_hash,
		api.genesis_hash,
		api.runtime_version.spec_version,
		api.runtime_version.transaction_version
	)
}

/// Wraps the supplied call in a sudo call
pub fn sudo_call<C: Encode + Clone>(metadata: &Metadata, call: C) -> ([u8; 2], C) {
	compose_call!(metadata, "Sudo", "sudo", call)
}

/// Wraps the supplied calls in a batch call
pub fn batch_call<C: Encode + Clone>(metadata: &Metadata, calls: Vec<C>) -> ([u8; 2], Vec<C>) {
	compose_call!(metadata, "Utility", "batch", calls)
}

/// Handles the potential case of a negative ceremony index CLI.
///
/// If negative: returns the `current_ceremony_index` - `ceremony_index`
/// If positive: returns `ceremony_index`
///
/// Panics when `ceremony_index` == 0, or when effective index would be negative.
///
pub fn into_effective_cindex(
	ceremony_index: i32,
	current_ceremony_index: CeremonyIndexType,
) -> CeremonyIndexType {
	match ceremony_index {
		i32::MIN..=-1 => current_ceremony_index - ceremony_index.abs() as u32,
		1..=i32::MAX => ceremony_index as CeremonyIndexType,
		0 => panic!("Zero not allowed as ceremony index"),
	}
}

/// Utils around key management for
pub mod keys {
	use crate::{AccountPublic, KEYSTORE_PATH};
	use encointer_node_notee_runtime::AccountId;
	use log::{debug, trace};
	use sp_application_crypto::sr25519;
	use sp_core::{crypto::Ss58Codec, Pair};
	use sp_runtime::traits::IdentifyAccount;
	use std::path::PathBuf;
	use substrate_client_keystore::LocalKeystore;

	/// Get the account id from public SS58 or from dev-seed
	pub fn get_accountid_from_str(account: &str) -> AccountId {
		debug!("getting AccountId from -{}-", account);
		match &account[..2] {
			"//" =>
				AccountPublic::from(sr25519::Pair::from_string(account, None).unwrap().public())
					.into_account(),
			_ => AccountPublic::from(sr25519::Public::from_ss58check(account).unwrap())
				.into_account(),
		}
	}

	/// Get a pair either from keyring (well-known keys) or from the store
	pub fn get_pair_from_str(account: &str) -> sr25519::AppPair {
		debug!("getting pair for {}", account);
		match &account[..2] {
			"//" => sr25519::AppPair::from_string(account, None).unwrap(),
			_ => {
				debug!("fetching from keystore at {}", &KEYSTORE_PATH);
				// open store without password protection
				let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None)
					.expect("store should exist");
				trace!("store opened");
				let pair = store
					.key_pair::<sr25519::AppPair>(
						&sr25519::Public::from_ss58check(account).unwrap().into(),
					)
					.unwrap();
				drop(store);
				pair.unwrap()
			},
		}
	}
}
