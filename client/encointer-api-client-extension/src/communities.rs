use crate::{Api, Result};
use encointer_primitives::communities::{CommunityIdentifier, Location};
use serde_json::json;
use substrate_api_client::ApiClientError;

pub trait CommunitiesApi {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>>;
}

impl CommunitiesApi for Api {
	fn get_locations(&self, cid: CommunityIdentifier) -> Result<Vec<Location>> {
		let req = json!({
		"method": "encointer_getLocations",
		"params": vec![cid],
		"jsonrpc": "2.0",
		"id": "1",
		});

		let locations = self.get_request(req.into())?.ok_or_else(|| {
			ApiClientError::Other(format!("No locations founds. Does the cid {} exist", cid).into())
		})?;

		serde_json::from_str(&locations).map_err(|e| ApiClientError::Other(e.into()))
	}
}
