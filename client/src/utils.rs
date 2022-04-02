use codec::Encode;
use encointer_node_notee_runtime::AccountId;
use encointer_primitives::{
	balances::{BalanceType, Demurrage},
	communities::{CommunityIdentifier, CommunityMetadata, Degree, Location},
	fixed::transcendental::ln,
	scheduler::CeremonyIndexType,
};
use geojson::GeoJson;
use log::debug;
use sp_application_crypto::sr25519;
use sp_core::Pair;
use substrate_api_client::{
	compose_extrinsic_offline, rpc::WsRpcClient, Api, UncheckedExtrinsicV4,
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

pub fn read_community_spec_from_file(path: &str) -> serde_json::Value {
	let spec_str = std::fs::read_to_string(path).unwrap();
	serde_json::from_str(&spec_str).unwrap()
}

/// Helper functions to handle the community
pub trait CommunitySpec {
	/// The community's locations.
	fn locations(&self) -> Vec<Location>;

	/// The community's bootstrappers.
	fn bootstrappers(&self) -> Vec<AccountId>;

	/// The community's metadata.
	fn metadata(&self) -> CommunityMetadata;

	/// The community field of the json
	fn community(&self) -> &serde_json::Value;

	/// The community's [CommunityIdentifier].
	fn community_identifier(&self) -> CommunityIdentifier;

	/// The community's demurrage if it has one set.
	fn demurrage(&self) -> Option<Demurrage>;

	/// The community's custom ceremony income if it has one set.
	fn ceremony_reward(&self) -> Option<BalanceType>;
}

impl CommunitySpec for serde_json::Value {
	fn locations(&self) -> Vec<Location> {
		let geoloc = GeoJson::from_json_value(self.clone()).unwrap();
		let mut loc = vec![];

		match geoloc {
			GeoJson::FeatureCollection(ref ctn) =>
				for feature in &ctn.features {
					let val = &feature.geometry.as_ref().unwrap().value;
					if let geojson::Value::Point(pt) = val {
						let l =
							Location { lon: Degree::from_num(pt[0]), lat: Degree::from_num(pt[1]) };
						loc.push(l);
						debug!("lon: {} lat {} => {:?}", pt[0], pt[1], l);
					}
				},
			_ => (),
		};

		loc
	}

	fn bootstrappers(&self) -> Vec<AccountId> {
		self["community"]["bootstrappers"]
			.as_array()
			.expect("bootstrappers must be array")
			.iter()
			.map(|a| keys::get_accountid_from_str(&a.as_str().unwrap()))
			.collect()
	}

	fn metadata(&self) -> CommunityMetadata {
		serde_json::from_value(self["community"]["meta"].clone()).unwrap()
	}

	fn community(&self) -> &serde_json::Value {
		&self["community"]
	}

	fn community_identifier(&self) -> CommunityIdentifier {
		CommunityIdentifier::new(self.locations()[0], self.bootstrappers()).unwrap()
	}

	fn demurrage(&self) -> Option<Demurrage> {
		match serde_json::from_value::<u64>(self["community"]["demurrage_halving_blocks"].clone()) {
			Ok(demurrage_halving_blocks) => {
				let demurrage_rate = ln::<BalanceType, BalanceType>(BalanceType::from_num(0.5))
					.unwrap()
					.checked_mul(BalanceType::from_num(-1))
					.unwrap()
					.checked_div(BalanceType::from_num(demurrage_halving_blocks))
					.unwrap();

				log::info!(
					"demurrage halving blocks: {} which translates to a rate of {} ",
					demurrage_halving_blocks,
					hex::encode(demurrage_rate.encode())
				);
				Some(demurrage_rate)
			},
			Err(_) => None,
		}
	}

	fn ceremony_reward(&self) -> Option<BalanceType> {
		match serde_json::from_value::<f64>(self["community"]["ceremony_income"].clone()) {
			Ok(reward) => {
				log::info!("ceremony income specified as {}", reward);
				Some(BalanceType::from_num(reward))
			},
			Err(_) => None,
		}
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
