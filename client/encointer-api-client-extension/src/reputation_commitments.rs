use crate::{Api, Result};
use encointer_node_notee_runtime::{AccountId, Hash};
use encointer_primitives::{
	ceremonies::CommunityCeremony,
	reputation_commitments::{DescriptorType, PurposeIdType},
};
use substrate_api_client::GetStorage;

#[maybe_async::maybe_async(?Send)]
pub trait ReputationCommitmentsApi {
	async fn get_commitment(
		&self,
		community_ceremony: &CommunityCeremony,
		purpose_account: (PurposeIdType, AccountId),
		maybe_at: Option<Hash>,
	) -> Result<Option<Option<Hash>>>;
	async fn get_current_purpose_id(&self, maybe_at: Option<Hash>)
		-> Result<Option<PurposeIdType>>;
	async fn get_purpose_descriptor(
		&self,
		purpose_id: PurposeIdType,
		maybe_at: Option<Hash>,
	) -> Result<Option<DescriptorType>>;
}

#[maybe_async::maybe_async(?Send)]
impl ReputationCommitmentsApi for Api {
	async fn get_commitment(
		&self,
		community_ceremony: &CommunityCeremony,
		purpose_account: (PurposeIdType, AccountId),
		maybe_at: Option<Hash>,
	) -> Result<Option<Option<Hash>>> {
		self.get_storage_double_map(
			"EncointerReputationCommitments",
			"Commitments",
			community_ceremony,
			purpose_account,
			maybe_at,
		)
		.await
	}
	async fn get_current_purpose_id(
		&self,
		maybe_at: Option<Hash>,
	) -> Result<Option<PurposeIdType>> {
		self.get_storage("EncointerReputationCommitments", "CurrentPurposeId", maybe_at)
			.await
	}
	async fn get_purpose_descriptor(
		&self,
		purpose_id: PurposeIdType,
		maybe_at: Option<Hash>,
	) -> Result<Option<DescriptorType>> {
		self.get_storage_map("EncointerReputationCommitments", "Purposes", purpose_id, maybe_at)
			.await
	}
}
