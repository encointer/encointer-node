use crate::{Api, Moment, Result};
use encointer_node_notee_runtime::Hash;
use encointer_primitives::{
	ceremonies::ReputationCountType,
	democracy::{ProposalIdType, Tally},
	reputation_commitments::PurposeIdType,
};
use std::time::Duration;
use substrate_api_client::GetStorage;

#[maybe_async::maybe_async(?Send)]
pub trait DemocracyApi {
	async fn get_proposal_lifetime(&self) -> Result<Duration>;
	async fn get_confirmation_period(&self) -> Result<Duration>;
	async fn get_min_turnout(&self) -> Result<ReputationCountType>;
	async fn get_tally(
		&self,
		proposal_id: ProposalIdType,
		maybe_at: Option<Hash>,
	) -> Result<Option<Tally>>;
	async fn get_purpose_id(
		&self,
		proposal_id: ProposalIdType,
		maybe_at: Option<Hash>,
	) -> Result<Option<PurposeIdType>>;
}

#[maybe_async::maybe_async(?Send)]
impl DemocracyApi for Api {
	async fn get_proposal_lifetime(&self) -> Result<Duration> {
		Ok(Duration::from_millis(
			self.get_constant::<Moment>("EncointerDemocracy", "ProposalLifetime").await?,
		))
	}
	async fn get_confirmation_period(&self) -> Result<Duration> {
		Ok(Duration::from_millis(
			self.get_constant::<Moment>("EncointerDemocracy", "ConfirmationPeriod").await?,
		))
	}
	async fn get_min_turnout(&self) -> Result<ReputationCountType> {
		self.get_constant("EncointerDemocracy", "MinTurnout").await
	}
	async fn get_tally(
		&self,
		proposal_id: ProposalIdType,
		maybe_at: Option<Hash>,
	) -> Result<Option<Tally>> {
		self.get_storage_map("EncointerDemocracy", "Tallies", proposal_id, maybe_at)
			.await
	}

	async fn get_purpose_id(
		&self,
		proposal_id: ProposalIdType,
		maybe_at: Option<Hash>,
	) -> Result<Option<PurposeIdType>> {
		self.get_storage_map("EncointerDemocracy", "PurposeIds", proposal_id, maybe_at)
			.await
	}
}
