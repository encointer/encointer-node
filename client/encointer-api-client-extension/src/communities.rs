use crate::{Api, Result};
use encointer_node_notee_runtime::Hash;
use encointer_primitives::{
	balances::{BalanceType, Demurrage},
	communities::{CidName, CommunityIdentifier, CommunityMetadata, GeoHash, Location},
};
use std::str::FromStr;
use substrate_api_client::{ac_compose_macros::rpc_params, rpc::Request, GetStorage};

#[maybe_async::maybe_async(?Send)]
pub trait CommunitiesApi {
	async fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
	async fn get_community_identifiers(
		&self,
		maybe_at: Option<Hash>,
	) -> Option<Vec<CommunityIdentifier>>;
	async fn get_nominal_income(
		&self,
		cid: CommunityIdentifier,
		maybe_at: Option<Hash>,
	) -> Option<BalanceType>;
	async fn get_demurrage_per_block(
		&self,
		cid: CommunityIdentifier,
		maybe_at: Option<Hash>,
	) -> Option<Demurrage>;
	async fn get_community_metadata(
		&self,
		cid: CommunityIdentifier,
		maybe_at: Option<Hash>,
	) -> Option<CommunityMetadata>;
	async fn get_locations_by_geohash(
		&self,
		cid: CommunityIdentifier,
		geo_hash: GeoHash,
		maybe_at: Option<Hash>,
	) -> Option<Vec<Location>>;
	async fn get_cid_names(&self) -> Option<Vec<CidName>>;
	async fn verify_cid(&self, cid: &str, maybe_at: Option<Hash>) -> CommunityIdentifier;
}

#[maybe_async::maybe_async(?Send)]
impl CommunitiesApi for Api {
	async fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>> {
		let locations = self
			.client()
			.request::<Vec<Location>>("encointer_getLocations", rpc_params![cid])
			.await?;
		Ok(locations)
	}
	async fn get_community_identifiers(
		&self,
		maybe_at: Option<Hash>,
	) -> Option<Vec<CommunityIdentifier>> {
		self.get_storage("EncointerCommunities", "CommunityIdentifiers", maybe_at)
			.await
			.unwrap()
	}

	async fn get_nominal_income(
		&self,
		cid: CommunityIdentifier,
		maybe_at: Option<Hash>,
	) -> Option<BalanceType> {
		self.get_storage_map("EncointerCommunities", "NominalIncome", cid, maybe_at)
			.await
			.unwrap()
	}

	async fn get_demurrage_per_block(
		&self,
		cid: CommunityIdentifier,
		maybe_at: Option<Hash>,
	) -> Option<Demurrage> {
		self.get_storage_map("EncointerBalances", "DemurragePerBlock", cid, maybe_at)
			.await
			.unwrap()
	}
	async fn get_community_metadata(
		&self,
		cid: CommunityIdentifier,
		maybe_at: Option<Hash>,
	) -> Option<CommunityMetadata> {
		self.get_storage_map("EncointerCommunities", "CommunityMetadata", cid, maybe_at)
			.await
			.unwrap()
	}

	async fn get_locations_by_geohash(
		&self,
		cid: CommunityIdentifier,
		geo_hash: GeoHash,
		maybe_at: Option<Hash>,
	) -> Option<Vec<Location>> {
		self.get_storage_double_map("EncointerCommunities", "Locations", cid, geo_hash, maybe_at)
			.await
			.unwrap()
	}

	/// This rpc needs to have offchain indexing enabled in the node.
	async fn get_cid_names(&self) -> Option<Vec<CidName>> {
		self.client().request("encointer_getAllCommunities", rpc_params![]).await.expect(
			"No communities returned. Are you running the node with `--enable-offchain-indexing true`?",
		)
	}

	async fn verify_cid(&self, cid: &str, maybe_at: Option<Hash>) -> CommunityIdentifier {
		let cids = self.get_community_identifiers(maybe_at).await.expect("no community registered");
		let cid = CommunityIdentifier::from_str(cid).unwrap();
		if !cids.contains(&cid) {
			panic!("cid {cid} does not exist on chain");
		}
		cid
	}
}
