use crate::{Api, Result};
use encointer_primitives::communities::{CommunityIdentifier, Location};
use substrate_api_client::{api::error::Error as ApiClientError, rpc::Request, RpcParams};

pub trait CommunitiesApi {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
}

impl CommunitiesApi for Api {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>> {
		let mut params = RpcParams::default();
		params.insert(cid).map_err(|_| {
			ApiClientError::Other(format!("Could not build the request using cid: {cid}").into())
		})?;

		let locations: String =
			self.client().request::<String>("encointer_getLocations", params).or_else(|_| {
				Err(ApiClientError::Other(
					format!("No locations founds. Does the cid {cid} exist").into(),
				))
			})?;

		serde_json::from_str(&locations).map_err(|e| ApiClientError::Other(e.into()))
	}
}
