use crate::{Api, Result};
use encointer_primitives::communities::{CommunityIdentifier, Location};
use substrate_api_client::{ac_compose_macros::rpc_params, rpc::Request};

#[maybe_async::maybe_async(?Send)]
pub trait CommunitiesApi {
	async fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
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
}
