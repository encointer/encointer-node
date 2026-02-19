use crate::Api;
use encointer_node_runtime::AccountId;
use encointer_primitives::communities::CommunityIdentifier;

use substrate_api_client::{ac_compose_macros::rpc_params, rpc::Request};

#[maybe_async::maybe_async(?Send)]
pub trait TreasuriesApi {
	async fn get_community_treasury_account_unchecked(
		&self,
		maybecid: Option<CommunityIdentifier>,
	) -> Option<AccountId>;
}

#[maybe_async::maybe_async(?Send)]
impl TreasuriesApi for Api {
	async fn get_community_treasury_account_unchecked(
		&self,
		maybecid: Option<CommunityIdentifier>,
	) -> Option<AccountId> {
		self.client()
			.request("encointer_getCommunityTreasuryAccountUnchecked", rpc_params![maybecid])
			.await
			.expect("Could not get treasury address...")
	}
}
