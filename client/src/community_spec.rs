use codec::Encode;
use encointer_node_notee_runtime::AccountId;
use encointer_primitives::{
	balances::{BalanceType, Demurrage},
	communities::{CommunityIdentifier, CommunityMetadata, Degree, Location},
	fixed::transcendental::ln,
};
use geojson::GeoJson;
use log::{debug, info};
use substrate_api_client::{compose_call, Metadata};

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

		if let GeoJson::FeatureCollection(ref ctn) = geoloc {
			for feature in &ctn.features {
				let val = &feature.geometry.as_ref().unwrap().value;
				if let geojson::Value::Point(pt) = val {
					let l = Location { lon: Degree::from_num(pt[0]), lat: Degree::from_num(pt[1]) };
					loc.push(l);
					debug!("lon: {} lat {} => {:?}", pt[0], pt[1], l);
				}
			}
		};

		loc
	}

	fn bootstrappers(&self) -> Vec<AccountId> {
		self["community"]["bootstrappers"]
			.as_array()
			.expect("bootstrappers must be array")
			.iter()
			.map(|a| crate::utils::keys::get_accountid_from_str(a.as_str().unwrap()))
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

type NewCommunityCall =
	([u8; 2], Location, Vec<AccountId>, CommunityMetadata, Option<Demurrage>, Option<BalanceType>);

/// Extracts all the info from `spec` to create a `new_community` call.
pub fn new_community_call<T: CommunitySpec>(spec: &T, metadata: &Metadata) -> NewCommunityCall {
	debug!("meta: {:?}", spec.community());

	let bootstrappers = spec.bootstrappers();

	let meta = spec.metadata();

	meta.validate().unwrap();
	info!("Metadata: {:?}", meta);

	info!("bootstrappers: {:?}", bootstrappers);
	info!("name: {}", meta.name);

	let maybe_demurrage = spec.demurrage();
	if maybe_demurrage.is_none() {
		info!("using default demurrage");
	};

	let maybe_income = spec.ceremony_reward();
	if maybe_income.is_none() {
		info!("using default income");
	}

	compose_call!(
		metadata,
		"EncointerCommunities",
		"new_community",
		spec.locations()[0],
		bootstrappers,
		meta,
		maybe_demurrage,
		maybe_income
	)
}

pub type AddLocationCall = ([u8; 2], CommunityIdentifier, Location);

/// Create an `add_location` call to be used in an extrinsic.
pub fn add_location_call(
	metadata: &Metadata,
	cid: CommunityIdentifier,
	loc: Location,
) -> AddLocationCall {
	compose_call!(metadata, "EncointerCommunities", "add_location", cid, loc)
}
