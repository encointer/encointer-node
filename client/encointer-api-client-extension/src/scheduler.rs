use crate::{Api, Moment, Result};
use encointer_node_runtime::Hash;
use encointer_primitives::{ceremonies::CeremonyIndexType, scheduler::CeremonyPhaseType};
use substrate_api_client::{api::error::Error as ApiClientError, GetStorage};

#[maybe_async::maybe_async(?Send)]
pub trait SchedulerApi {
	async fn get_ceremony_index(&self, maybe_at: Option<Hash>) -> CeremonyIndexType;
	async fn get_current_phase(&self, maybe_at: Option<Hash>) -> Result<CeremonyPhaseType>;
	async fn get_next_phase_timestamp(&self, maybe_at: Option<Hash>) -> Result<Moment>;
	async fn get_phase_duration(
		&self,
		phase: CeremonyPhaseType,
		maybe_at: Option<Hash>,
	) -> Result<Moment>;
	async fn get_cycle_duration(&self, maybe_at: Option<Hash>) -> Result<Moment>;
	async fn get_start_of_attesting_phase(&self, maybe_at: Option<Hash>) -> Result<Moment>;
}

#[maybe_async::maybe_async(?Send)]
impl SchedulerApi for Api {
	async fn get_ceremony_index(&self, maybe_at: Option<Hash>) -> CeremonyIndexType {
		self.get_storage("EncointerScheduler", "CurrentCeremonyIndex", maybe_at)
			.await
			.unwrap()
			.expect("ceremony index not set")
	}

	async fn get_current_phase(&self, maybe_at: Option<Hash>) -> Result<CeremonyPhaseType> {
		self.get_storage("EncointerScheduler", "CurrentPhase", maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("Couldn't get CurrentPhase".into()))
	}

	async fn get_next_phase_timestamp(&self, maybe_at: Option<Hash>) -> Result<Moment> {
		self.get_storage("EncointerScheduler", "NextPhaseTimestamp", maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("Couldn't get NextPhaseTimestamp".into()))
	}

	async fn get_phase_duration(
		&self,
		phase: CeremonyPhaseType,
		maybe_at: Option<Hash>,
	) -> Result<Moment> {
		self.get_storage_map("EncointerScheduler", "PhaseDurations", phase, maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("Couldn't get PhaseDuration".into()))
	}

	async fn get_cycle_duration(&self, maybe_at: Option<Hash>) -> Result<Moment> {
		let parts = tokio::try_join!(
			self.get_phase_duration(CeremonyPhaseType::Registering, maybe_at),
			self.get_phase_duration(CeremonyPhaseType::Assigning, maybe_at),
			self.get_phase_duration(CeremonyPhaseType::Attesting, maybe_at)
		)?;
		Ok(parts.0 + parts.1 + parts.2)
	}

	async fn get_start_of_attesting_phase(&self, maybe_at: Option<Hash>) -> Result<Moment> {
		let next_phase_timestamp = self.get_next_phase_timestamp(maybe_at).await?;

		match self.get_current_phase(maybe_at).await? {
			CeremonyPhaseType::Assigning => Ok(next_phase_timestamp), // - next_phase_timestamp.rem(ONE_DAY),
			CeremonyPhaseType::Attesting => {
				self.get_phase_duration(CeremonyPhaseType::Attesting, maybe_at)
					.await
					.map(|dur| next_phase_timestamp - dur) //- next_phase_timestamp.rem(ONE_DAY)
			},
			CeremonyPhaseType::Registering => Err(ApiClientError::Other(
				"ceremony phase must be Assigning or Attesting to request meetup location.".into(),
			)),
		}
	}
}
