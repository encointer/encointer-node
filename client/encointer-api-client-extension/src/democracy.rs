use crate::{Api, Moment, Result};
use std::time::Duration;
use substrate_api_client::GetStorage;

#[maybe_async::maybe_async(?Send)]
pub trait DemocracyApi {
	async fn get_proposal_lifetime(&self) -> Result<Duration>;
	async fn get_confirmation_period(&self) -> Result<Duration>;
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
}
