use crate::{Api, Moment, Result};
use encointer_primitives::scheduler::CeremonyPhaseType;
use substrate_api_client::{api::error::Error as ApiClientError, GetStorage};

pub trait SchedulerApi {
	async fn get_current_phase(&self) -> Result<CeremonyPhaseType>;
	async fn get_next_phase_timestamp(&self) -> Result<Moment>;
	async fn get_phase_duration(&self, phase: CeremonyPhaseType) -> Result<Moment>;
	async fn get_start_of_attesting_phase(&self) -> Result<Moment>;
}

impl SchedulerApi for Api {
	async fn get_current_phase(&self) -> Result<CeremonyPhaseType> {
		self.get_storage("EncointerScheduler", "CurrentPhase", None)
			.await?
			.ok_or_else(|| ApiClientError::Other("Couldn't get CurrentPhase".into()))
	}

	async fn get_next_phase_timestamp(&self) -> Result<Moment> {
		self.get_storage("EncointerScheduler", "NextPhaseTimestamp", None)
			.await?
			.ok_or_else(|| ApiClientError::Other("Couldn't get NextPhaseTimestamp".into()))
	}

	async fn get_phase_duration(&self, phase: CeremonyPhaseType) -> Result<Moment> {
		self.get_storage_map("EncointerScheduler", "PhaseDurations", phase, None)
			.await?
			.ok_or_else(|| ApiClientError::Other("Couldn't get PhaseDuration".into()))
	}

	async fn get_start_of_attesting_phase(&self) -> Result<Moment> {
		let next_phase_timestamp = self.get_next_phase_timestamp().await?;

		match self.get_current_phase().await? {
			CeremonyPhaseType::Assigning => Ok(next_phase_timestamp), // - next_phase_timestamp.rem(ONE_DAY),
			CeremonyPhaseType::Attesting => {
				self.get_phase_duration(CeremonyPhaseType::Attesting)
					.await
					.map(|dur| next_phase_timestamp - dur) //- next_phase_timestamp.rem(ONE_DAY)
			},
			CeremonyPhaseType::Registering => Err(ApiClientError::Other(
				"ceremony phase must be Assigning or Attesting to request meetup location.".into(),
			)),
		}
	}
}
