use crate::{
	cli::Cli,
	commands::encointer_core::{get_asset_fee_details, get_community_balance},
	exit_code, BalanceType,
};
use encointer_api_client_extension::{Api, EncointerXt};
use encointer_node_runtime::AccountId;
use encointer_primitives::{balances::EncointerBalanceConverter, scheduler::CeremonyIndexType};
use log::{debug, error, info};
use parity_scale_codec::{Compact, Encode};
use sp_core::H256;
use sp_runtime::traits::Convert;
use std::str::FromStr;
use substrate_api_client::{
	ac_compose_macros::compose_call,
	ac_node_api::Metadata,
	ac_primitives::Bytes,
	api::{error::Error as ApiClientError, rpc_api::state::GetStorage},
	rpc::JsonrpseeClient,
	GetAccountInformation, GetBalance, GetTransactionPayment, Result, SubmitAndWatch, XtStatus,
};

pub async fn get_chain_api(cli: &Cli) -> Api {
	let url = format!("{}:{}", cli.node_url, cli.node_port);
	debug!("connecting to {}", url);
	let client = JsonrpseeClient::new(&url).await.expect("node URL is incorrect");
	Api::new(client).await.unwrap()
}

/// Creates a signed extrinsic from a call
///
/// Panics if no signer is set.
pub async fn xt<C: Encode + Clone>(api: &Api, call: C) -> EncointerXt<C> {
	let nonce = api.get_nonce().await.unwrap();
	api.compose_extrinsic_offline(call, nonce)
}

/// Wraps the supplied call in a sudo call
pub fn sudo_call<C: Encode + Clone>(metadata: &Metadata, call: C) -> ([u8; 2], C) {
	compose_call!(metadata, "Sudo", "sudo", call).unwrap()
}

pub type BatchCall<C> = ([u8; 2], Vec<C>);
/// Wraps the supplied calls in a batch call
pub fn batch_call<C: Encode + Clone>(metadata: &Metadata, calls: Vec<C>) -> BatchCall<C> {
	compose_call!(metadata, "Utility", "batch", calls).unwrap()
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum CallWrapping {
	None,
	Sudo,
	Collective,
}

impl FromStr for CallWrapping {
	type Err = &'static str;

	fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
		match value {
			"none" => Ok(CallWrapping::None),
			"sudo" => Ok(CallWrapping::Sudo),
			"collective" => Ok(CallWrapping::Collective),
			_ => Err("unknown call wrapping type"),
		}
	}
}

/// ([pallet_index, call_index], threshold, Proposal,length_bound)
///
/// `threshold` is the number of members. threshold < 1 will make the proposal be executed directly.
/// `length_bound` must be >= `Proposal.encode().len() + (size_of::<u32>() == 4)`
type CollectiveProposeCall<Proposal> = ([u8; 2], Compact<u32>, Proposal, Compact<u32>);

/// Creates a council propose call
pub fn collective_propose_call<Proposal: Encode>(
	metadata: &Metadata,
	threshold: u32,
	proposal: Proposal,
) -> CollectiveProposeCall<Proposal> {
	let length_bound = proposal.encode().len() as u32 + 4;
	compose_call!(
		metadata,
		"Collective",
		"propose",
		Compact(threshold),
		proposal,
		Compact(length_bound)
	)
	.unwrap()
}
pub async fn get_councillors(api: &Api) -> Result<Vec<AccountId>> {
	api.get_storage("Membership", "Members", None)
		.await?
		.ok_or_else(|| ApiClientError::Other("Couldn't get councillors".into()))
}

pub async fn send_and_wait_for_in_block<C: Encode>(
	api: &Api,
	xt: EncointerXt<C>,
	tx_payment_cid: Option<&str>,
) -> Option<H256> {
	ensure_payment(api, &xt.encode().into(), tx_payment_cid).await;
	let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap();
	info!("[+] Transaction got included in Block: {:?}\n", report.block_hash.unwrap());
	Some(report.extrinsic_hash)
}

pub async fn send_and_wait_for_finalized<C: Encode>(
	api: &Api,
	xt: EncointerXt<C>,
	tx_payment_cid: Option<&str>,
) -> Option<H256> {
	ensure_payment(api, &xt.encode().into(), tx_payment_cid).await;
	let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Finalized).await.unwrap();
	info!("[+] Transaction got finalized in Block: {:?}\n", report.block_hash.unwrap());
	Some(report.extrinsic_hash)
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
	if metadata.pallet_by_name("Sudo").is_some() {
		info!("'Sudo' pallet found on chain. Will send privileged xt's as sudo");
		true
	} else {
		info!("'Sudo' pallet not found on chain. Will send privileged xt's as a council-proposal");
		false
	}
}

/// Checks if the account has sufficient funds. Exits the process if not.
pub async fn ensure_payment(api: &Api, encoded_xt: &Bytes, tx_payment_cid: Option<&str>) {
	if let Some(cid_str) = tx_payment_cid {
		ensure_payment_cc(api, cid_str, encoded_xt).await;
	} else {
		ensure_payment_native(api, encoded_xt).await;
	}
}

async fn ensure_payment_cc(api: &Api, cid_str: &str, encoded_xt: &Bytes) {
	let balance: BalanceType =
		get_community_balance(api, cid_str, api.signer_account().unwrap(), None).await;

	let fee: BalanceType = get_asset_fee_details(api, cid_str, encoded_xt)
		.await
		.unwrap()
		.inclusion_fee
		.map(|details| details.base_fee.into_u256().as_u128())
		.map(EncointerBalanceConverter::convert)
		.unwrap_or_default();

	if balance < fee {
		error!("insufficient funds in CC: fee: {} bal: {:?}", fee, balance);
		std::process::exit(exit_code::FEE_PAYMENT_FAILED);
	}
	debug!("account can pay fees in CC: fee: {} bal: {}", fee, balance);
}

async fn ensure_payment_native(api: &Api, encoded_xt: &Bytes) {
	let signer_balance = match api.get_account_data(api.signer_account().unwrap()).await.unwrap() {
		Some(bal) => bal.free,
		None => {
			error!("account does not exist on chain");
			std::process::exit(exit_code::FEE_PAYMENT_FAILED);
		},
	};
	let fee = api
		.get_fee_details(encoded_xt, None)
		.await
		.unwrap()
		.unwrap()
		.inclusion_fee
		.map_or_else(|| 0, |details| details.base_fee);
	let ed = api.get_existential_deposit().await.unwrap();
	if signer_balance < fee + ed {
		error!("insufficient funds: fee: {} ed: {} bal: {:?}", fee, ed, signer_balance);
		std::process::exit(exit_code::FEE_PAYMENT_FAILED);
	}
	debug!("account can pay native fees: fee: {} ed: {} bal: {}", fee, ed, signer_balance);
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
		i32::MIN..=-1 => current_ceremony_index - ceremony_index.unsigned_abs(),
		1..=i32::MAX => ceremony_index as CeremonyIndexType,
		0 => panic!("Zero not allowed as ceremony index"),
	}
}

/// Simple blob to hold a call in encoded format.
///
/// Useful for managing a set of extrinsic with different calls without having problems with rust's
/// type system.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct OpaqueCall(pub Vec<u8>);

impl OpaqueCall {
	/// Convert a call to an `OpaqueCall`.
	pub fn from_tuple<C: Encode>(call: &C) -> Self {
		OpaqueCall(call.encode())
	}
}

impl Encode for OpaqueCall {
	fn encode(&self) -> Vec<u8> {
		self.0.clone()
	}
}

/// Utils around key management for
pub mod keys {
	use ac_keystore::LocalKeystore;
	use encointer_node_runtime::{AccountId, Signature};
	use log::{debug, trace};
	use sp_application_crypto::sr25519;
	use sp_core::{
		crypto::{KeyTypeId, Ss58Codec},
		Pair,
	};
	use sp_runtime::traits::{IdentifyAccount, Verify};
	use std::path::PathBuf;

	type AccountPublic = <Signature as Verify>::Signer;

	/// Key type for the generic Sr25519 key.
	pub const SR25519: KeyTypeId = KeyTypeId(*b"sr25");

	pub const KEYSTORE_PATH: &str = "my_keystore";

	/// Get the account id from public SS58 or from dev-seed.
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

	/// Get a pair either from keyring (well-known keys) or from the store.
	pub fn get_pair_from_str(account: &str) -> sr25519::AppPair {
		debug!("getting pair for {}", account);
		match &account[..2] {
			"//" => sr25519::AppPair::from_string(account, None).unwrap(),
			"0x" => sr25519::AppPair::from_string_with_seed(account, None).unwrap().0,
			_ => {
				if sr25519::Public::from_ss58check(account).is_err() {
					// could be mnemonic phrase
					return sr25519::AppPair::from_string_with_seed(account, None).unwrap().0;
				}
				debug!("fetching from keystore at {}", &KEYSTORE_PATH);
				// open store without password protection
				let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None)
					.expect("store should exist");
				trace!("store opened");
				let maybe_pair = store
					.key_pair::<sr25519::AppPair>(
						&sr25519::Public::from_ss58check(account).unwrap().into(),
					)
					.unwrap();
				drop(store);
				match maybe_pair {
					Some(pair) => pair,
					None => panic!("account not in keystore"),
				}
			},
		}
	}
}
