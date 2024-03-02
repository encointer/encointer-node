use crate::{Api, Result};
use encointer_primitives::communities::{CommunityIdentifier, Location};
use substrate_api_client::{ac_compose_macros::rpc_params, rpc::Request};

pub trait CommunitiesApi {
	async fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
}

impl CommunitiesApi for Api {
	async fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>> {
		let locations = self
			.client()
			.request::<Vec<Location>>("encointer_getLocations", rpc_params![cid])?;
		Ok(locations)
	}
}
