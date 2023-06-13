use crate::{Api, Result};
use encointer_primitives::communities::{CommunityIdentifier, Location};
use substrate_api_client::{rpc::Request, rpc_params};

pub trait CommunitiesApi {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
}

impl CommunitiesApi for Api {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>> {
		let locations = self
			.client()
			.request::<Vec<Location>>("encointer_getLocations", rpc_params![cid])?;
		Ok(locations)
	}
}
