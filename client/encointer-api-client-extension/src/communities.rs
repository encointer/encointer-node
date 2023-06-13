use crate::{Api, Result};
use encointer_primitives::communities::{CommunityIdentifier, Location};
use substrate_api_client::{api::error::Error as ApiClientError, rpc::Request, rpc_params};

pub trait CommunitiesApi {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
}

impl CommunitiesApi for Api {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>> {
		let locations = self
			.client()
			.request::<Option<Vec<Location>>>("encointer_getLocations", rpc_params![cid])?
			.ok_or_else(|| {
				ApiClientError::Other(
					format!("No locations founds. Does the cid {cid} exist").into(),
				)
			})?;
		Ok(locations)
	}
}
