use crate::{Api, Result};
use encointer_primitives::scheduler::CeremonyPhaseType;
use substrate_api_client::{ApiClientError, Moment};

pub trait SchedulerApi {
	fn get_current_phase(&self) -> Result<CeremonyPhaseType>;
	fn get_next_phase_timestamp(&self) -> Result<Moment>;
	fn get_phase_duration(&self, phase: CeremonyPhaseType) -> Result<Moment>;
	fn get_start_of_attesting_phase(&self) -> Result<Moment>;
}

impl SchedulerApi for Api {
	fn get_current_phase(&self) -> Result<CeremonyPhaseType> {
		self.get_storage_value("EncointerScheduler", "CurrentPhase", None)?
			.ok_or_else(|| ApiClientError::Other("Couldn't get CurrentPhase".into()))
	}

	fn get_next_phase_timestamp(&self) -> Result<Moment> {
		self.get_storage_value("EncointerScheduler", "NextPhaseTimestamp", None)?
			.ok_or_else(|| ApiClientError::Other("Couldn't get NextPhaseTimestamp".into()))
	}

	fn get_phase_duration(&self, phase: CeremonyPhaseType) -> Result<Moment> {
		self.get_storage_map("EncointerScheduler", "PhaseDurations", phase, None)?
			.ok_or_else(|| ApiClientError::Other("Couldn't get PhaseDuration".into()))
	}

	fn get_start_of_attesting_phase(&self) -> Result<Moment> {
		let next_phase_timestamp = self.get_next_phase_timestamp()?;

		match self.get_current_phase()? {
			CeremonyPhaseType::Assigning => Ok(next_phase_timestamp), // - next_phase_timestamp.rem(ONE_DAY),
			CeremonyPhaseType::Attesting => {
				self.get_phase_duration(CeremonyPhaseType::Attesting)
					.map(|dur| next_phase_timestamp - dur) //- next_phase_timestamp.rem(ONE_DAY)
			},
			CeremonyPhaseType::Registering => Err(ApiClientError::Other(
				"ceremony phase must be Assigning or Attesting to request meetup location.".into(),
			)),
		}
	}
}
