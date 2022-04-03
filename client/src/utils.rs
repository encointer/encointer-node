use crate::exit_code;
use codec::Encode;
use encointer_primitives::scheduler::CeremonyIndexType;
use log::{debug, error, info};
use sp_application_crypto::sr25519;
use sp_core::{Pair, H256};
use substrate_api_client::{
	compose_call, compose_extrinsic, compose_extrinsic_offline, rpc::WsRpcClient, Api, Metadata,
	UncheckedExtrinsicV4, XtStatus,
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

/// Creates a signed extrinsic from a call
///
/// Panics if no signer is set.
pub fn xt<C: Encode + Clone>(
	api: &Api<sr25519::Pair, WsRpcClient>,
	call: C,
) -> UncheckedExtrinsicV4<C> {
	let nonce = api.get_nonce().unwrap();
	offline_xt(api, call, nonce)
}

/// Wraps the supplied call in a sudo call
pub fn sudo_call<C: Encode + Clone>(metadata: &Metadata, call: C) -> ([u8; 2], C) {
	compose_call!(metadata, "Sudo", "sudo", call)
}

pub fn sudo_xt<C: Encode + Clone>(
	api: &Api<sr25519::Pair, WsRpcClient>,
	call: C,
) -> UncheckedExtrinsicV4<([u8; 2], C)> {
	compose_extrinsic!(api, "Sudo", "sudo", call)
}

/// Wraps the supplied calls in a batch call
pub fn batch_call<C: Encode + Clone>(metadata: &Metadata, calls: Vec<C>) -> ([u8; 2], Vec<C>) {
	compose_call!(metadata, "Utility", "batch", calls)
}

/// ([pallet_index, call_index], threshold, Proposal,length_bound)
///
/// `threshold` is the number of members. threshold < 1 will make the proposal be executed directly.
/// `length_bound` must be >= `Proposal.encode().len() + (size_of::<u32>() == 4)`
type CollectiveProposeCall<Proposal> = ([u8; 2], u32, Proposal, u32);

/// Creates a council propose call
pub fn collective_propose_call<Proposal: Encode>(
	metadata: &Metadata,
	threshold: u32,
	proposal: Proposal,
) -> CollectiveProposeCall<Proposal> {
	let length_bound = proposal.encode().len() as u32 + 4;
	compose_call!(metadata, "Collective", "propose", threshold, proposal, length_bound)
}

pub fn send_and_wait_for_in_block<C: Encode>(
	api: &Api<sr25519::Pair, WsRpcClient>,
	xt: UncheckedExtrinsicV4<C>,
) -> Option<H256> {
	ensure_payment(&api, &xt.hex_encode());
	let tx_hash = api.send_extrinsic(xt.hex_encode(), XtStatus::InBlock).unwrap();
	info!("[+] Transaction got included. Hash: {:?}\n", tx_hash);

	tx_hash
}

/// Prints the raw call to be supplied with js/apps.
pub fn print_raw_call<Call: Encode>(name: &str, call: &Call) {
	info!("{}: 0x{}", name, hex::encode(call.encode()));
}

/// Checks if the sudo pallet exists on chain.
///
/// This will implicitly distinguish between solo-chain (sudo exists) and parachain
/// (sudo doesn't exist).
pub fn contains_sudo_pallet(metadata: &Metadata) -> bool {
	if metadata.pallet("Sudo").is_ok() {
		info!("'Sudo' pallet found on chain. Will send privileged xt's as sudo");
		true
	} else {
		info!("'Sudo' pallet not found on chain. Will send privileged xt's as a council-proposal");
		false
	}
}

/// Checks if the account has sufficient funds. Exits the process if not.
pub fn ensure_payment(api: &Api<sr25519::Pair, WsRpcClient>, xt: &str) {
	let signer_balance = match api.get_account_data(&api.signer_account().unwrap()).unwrap() {
		Some(bal) => bal.free,
		None => {
			error!("account does not exist on chain");
			std::process::exit(exit_code::FEE_PAYMENT_FAILED);
		},
	};
	let fee = api
		.get_fee_details(xt, None)
		.unwrap()
		.unwrap()
		.inclusion_fee
		.map_or_else(|| 0, |details| details.base_fee);
	let ed = api.get_existential_deposit().unwrap();
	if signer_balance < fee + ed {
		error!("insufficient funds: fee: {} ed: {} bal: {:?}", fee, ed, signer_balance);
		std::process::exit(exit_code::FEE_PAYMENT_FAILED);
	}
	debug!("account can pay fees: fee: {} ed: {} bal: {}", fee, ed, signer_balance);
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
