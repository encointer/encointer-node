use crate::Api;
use encointer_node_runtime::AccountId;
use encointer_primitives::{
	bazaar::{Business, BusinessIdentifier, OfferingData},
	communities::CommunityIdentifier,
};

use substrate_api_client::{ac_compose_macros::rpc_params, rpc::Request};

#[maybe_async::maybe_async(?Send)]
pub trait BazaarApi {
	async fn get_businesses(&self, cid: CommunityIdentifier) -> Option<Vec<Business<AccountId>>>;
	async fn get_offerings(&self, cid: CommunityIdentifier) -> Option<Vec<OfferingData>>;
	async fn get_offerings_for_business(
		&self,
		cid: CommunityIdentifier,
		account_id: AccountId,
	) -> Option<Vec<OfferingData>>;
}

#[maybe_async::maybe_async(?Send)]
impl BazaarApi for Api {
	async fn get_businesses(&self, cid: CommunityIdentifier) -> Option<Vec<Business<AccountId>>> {
		self.client()
			.request("encointer_bazaarGetBusinesses", rpc_params![cid])
			.await
			.expect("Could not find any businesses...")
	}

	async fn get_offerings(&self, cid: CommunityIdentifier) -> Option<Vec<OfferingData>> {
		self.client()
			.request("encointer_bazaarGetOfferings", rpc_params![cid])
			.await
			.expect("Could not find any business offerings...")
	}

	async fn get_offerings_for_business(
		&self,
		cid: CommunityIdentifier,
		account_id: AccountId,
	) -> Option<Vec<OfferingData>> {
		let b_id = BusinessIdentifier::new(cid, account_id);
		self.client()
			.request("encointer_bazaarGetOfferingsForBusiness", rpc_params![b_id])
			.await
			.expect("Could not find any business offerings...")
	}
}
