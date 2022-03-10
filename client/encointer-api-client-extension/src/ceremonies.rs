use crate::{Api, CommunitiesApi, Result, SchedulerApi};
use encointer_ceremonies_assignment::{
	assignment_fn_inverse, meetup_index, meetup_location, meetup_time,
};
use encointer_primitives::{
	ceremonies::{
		Assignment, AssignmentCount, CommunityCeremony, MeetupIndexType, ParticipantIndexType,
	},
	communities::Location,
};
use log::warn;
use substrate_api_client::{AccountId, ApiClientError, Moment};

pub const ENCOINTER_CEREMONIES: &'static str = "EncointerCeremonies";

pub trait CeremoniesApi {
	fn get_assignments(&self, community_ceremony: &CommunityCeremony) -> Result<Assignment>;
	fn get_assignment_counts(
		&self,
		community_ceremony: &CommunityCeremony,
	) -> Result<AssignmentCount>;

	fn get_bootstrapper(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>>;

	fn get_reputable(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>>;

	fn get_endorsee(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>>;

	fn get_newbie(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>>;

	fn get_meetup_count(&self, community_ceremony: &CommunityCeremony) -> Result<MeetupIndexType>;

	fn get_meetup_index(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
	) -> Result<Option<MeetupIndexType>>;

	fn get_meetup_location(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Option<Location>>;

	fn get_meetup_participants(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Vec<AccountId>>;

	fn get_meetup_time_offset(&self) -> Result<Option<Moment>>;

	fn get_meetup_time(&self, location: Location, one_day: Moment) -> Result<Moment>;
}

impl CeremoniesApi for Api {
	fn get_assignments(&self, community_ceremony: &CommunityCeremony) -> Result<Assignment> {
		self.get_storage_map(ENCOINTER_CEREMONIES, "Assignments", community_ceremony, None)?
			.ok_or_else(|| ApiClientError::Other("Assignments don't exist".into()))
	}

	fn get_assignment_counts(
		&self,
		community_ceremony: &CommunityCeremony,
	) -> Result<AssignmentCount> {
		self.get_storage_map(ENCOINTER_CEREMONIES, "AssignmentCounts", community_ceremony, None)?
			.ok_or_else(|| ApiClientError::Other("AssignmentCounts not found".into()))
	}

	fn get_bootstrapper(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"BootstrapperRegistry",
			community_ceremony,
			p,
			None,
		)
	}

	fn get_reputable(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"ReputableRegistry",
			community_ceremony,
			p,
			None,
		)
	}

	fn get_endorsee(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"EndorseeRegistry",
			community_ceremony,
			p,
			None,
		)
	}

	fn get_newbie(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"NewbieRegistry",
			community_ceremony,
			p,
			None,
		)
	}

	fn get_meetup_count(&self, community_ceremony: &CommunityCeremony) -> Result<MeetupIndexType> {
		Ok(self
			.get_storage_map(ENCOINTER_CEREMONIES, "MeetupCount", community_ceremony, None)?
			.unwrap_or(0))
	}

	fn get_meetup_index(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
	) -> Result<Option<MeetupIndexType>> {
		let meetup_count = self.get_meetup_count(community_ceremony)?;

		if meetup_count == 0 {
			warn!("Meetup Count is 0.");
			return Ok(None)
		}

		let assignments = self.get_assignments(community_ceremony)?;

		// Some helper queries to make below code more readable.
		let bootstrapper_count = || -> Result<ParticipantIndexType> {
			Ok(self.get_assignment_counts(community_ceremony)?.bootstrappers)
		};
		let index_query = |storage_key| -> Result<Option<ParticipantIndexType>> {
			self.get_storage_double_map(
				ENCOINTER_CEREMONIES,
				storage_key,
				community_ceremony,
				account_id,
				None,
			)
		};
		let meetup_index_fn =
			|p_index, assignment_params| meetup_index(p_index, assignment_params, meetup_count);

		// Finally get the meetup index

		if let Some(p_index) = index_query("BootstrapperIndex")? {
			return Ok(meetup_index_fn(p_index - 1, assignments.bootstrappers_reputables))
		} else if let Some(p_index) = index_query("ReputableIndex")? {
			return Ok(meetup_index_fn(
				p_index - 1 + bootstrapper_count()?,
				assignments.bootstrappers_reputables,
			))
		} else if let Some(p_index) = index_query("EndorseeIndex")? {
			return Ok(meetup_index_fn(p_index - 1, assignments.endorsees))
		} else if let Some(p_index) = index_query("NewbieIndex")? {
			return Ok(meetup_index_fn(p_index - 1, assignments.newbies))
		}

		Ok(None)
	}

	fn get_meetup_location(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Option<Location>> {
		let locations = self.get_locations(community_ceremony.0)?;
		let location_assignment_params = self.get_assignments(&community_ceremony)?.locations;

		Ok(meetup_location(meetup_index, locations, location_assignment_params))
	}

	fn get_meetup_participants(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Vec<AccountId>> {
		let meetup_index_zero_based = meetup_index - 1;
		let meetup_count = self.get_meetup_count(community_ceremony)?;

		if meetup_index_zero_based > meetup_count {
			return Err(ApiClientError::Other(
				format!(
					"Invalid meetup index > meetup count: {}, {}",
					meetup_index_zero_based, meetup_count
				)
				.into(),
			))
		}

		let params = self.get_assignments(community_ceremony)?;
		let assigned = self.get_assignment_counts(community_ceremony)?;

		let bootstrappers_reputables = assignment_fn_inverse(
			meetup_index_zero_based,
			params.bootstrappers_reputables,
			meetup_count,
			assigned.bootstrappers + assigned.reputables,
		)
		.into_iter()
		.filter_map(|p_index| {
			get_bootstrapper_or_reputable(self, community_ceremony, p_index, &assigned)
				.ok()
				.flatten()
		});

		let endorsees = assignment_fn_inverse(
			meetup_index_zero_based,
			params.endorsees,
			meetup_count,
			assigned.endorsees,
		)
		.into_iter()
		.filter(|p| p < &assigned.endorsees)
		.filter_map(|p| self.get_endorsee(community_ceremony, &(p + 1)).ok().flatten());

		let newbies = assignment_fn_inverse(
			meetup_index_zero_based,
			params.newbies,
			meetup_count,
			assigned.newbies,
		)
		.into_iter()
		.filter(|p| p < &assigned.newbies)
		.filter_map(|p| self.get_newbie(community_ceremony, &(p + 1)).ok().flatten());

		Ok(bootstrappers_reputables.chain(endorsees).chain(newbies).collect())
	}

	fn get_meetup_time_offset(&self) -> Result<Option<Moment>> {
		self.get_storage_value(ENCOINTER_CEREMONIES, "MeetupTimeOffset", None)
	}

	fn get_meetup_time(&self, location: Location, one_day: Moment) -> Result<Moment> {
		let attesting_start = self.get_start_of_attesting_phase()?;
		let offset = self.get_meetup_time_offset()?.unwrap_or(0);

		Ok(meetup_time(location, attesting_start, one_day, offset))
	}
}

fn get_bootstrapper_or_reputable(
	api: &Api,
	community_ceremony: &CommunityCeremony,
	p_index: ParticipantIndexType,
	assigned: &AssignmentCount,
) -> Result<Option<AccountId>> {
	if p_index < assigned.bootstrappers {
		return api.get_bootstrapper(community_ceremony, &(p_index + 1))
	} else if p_index < assigned.bootstrappers + assigned.reputables {
		return api.get_reputable(community_ceremony, &(p_index - assigned.bootstrappers + 1))
	}

	Ok(None)
}
