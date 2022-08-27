use crate::{Api, CommunitiesApi, Result, SchedulerApi};
use encointer_ceremonies_assignment::{
	assignment_fn_inverse, meetup_index, meetup_location, meetup_time,
};
use encointer_primitives::{
	ceremonies::{
		Assignment, AssignmentCount, CommunityCeremony, MeetupIndexType, MeetupTimeOffsetType,
		ParticipantIndexType,
	},
	communities::Location,
};
use log::warn;
use serde::{Deserialize, Serialize};
use substrate_api_client::{AccountId, ApiClientError, Moment};

pub const ENCOINTER_CEREMONIES: &'static str = "EncointerCeremonies";

// same as in runtime, but we did not want to import the runtime here.
pub const ONE_DAY: Moment = 86_400_000;

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

	fn get_registration(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
	) -> Result<Registration>;

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

	fn get_meetup_time_offset(&self) -> Result<Option<MeetupTimeOffsetType>>;

	fn get_meetup_time(&self, location: Location, one_day: Moment) -> Result<Moment>;

	fn get_community_ceremony_stats(
		&self,
		community_ceremony: CommunityCeremony,
	) -> Result<CommunityCeremonyStats>;
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

	fn get_registration(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
	) -> Result<Registration> {
		let index_query = |storage_key| -> Result<Option<ParticipantIndexType>> {
			self.get_storage_double_map(
				ENCOINTER_CEREMONIES,
				storage_key,
				community_ceremony,
				&account_id,
				None,
			)
		};

		if let Some(p_index) = index_query("BootstrapperIndex")? {
			return Ok(Registration::new(p_index, RegistrationType::Bootstrapper))
		} else if let Some(p_index) = index_query("ReputableIndex")? {
			return Ok(Registration::new(p_index, RegistrationType::Reputable))
		} else if let Some(p_index) = index_query("EndorseeIndex")? {
			return Ok(Registration::new(p_index, RegistrationType::Endorsee))
		} else if let Some(p_index) = index_query("NewbieIndex")? {
			return Ok(Registration::new(p_index, RegistrationType::Newbie))
		}

		Err(ApiClientError::Other(
			format!("Could not get participant index for {:?}", account_id).into(),
		))
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

		let registration = self.get_registration(community_ceremony, account_id)?;

		let meetup_index_fn =
			|p_index, assignment_params| meetup_index(p_index, assignment_params, meetup_count);

		// Finally get the meetup index

		match registration.registration_type {
			RegistrationType::Bootstrapper =>
				Ok(meetup_index_fn(registration.index - 1, assignments.bootstrappers_reputables)),
			RegistrationType::Reputable => Ok(meetup_index_fn(
				registration.index - 1 + bootstrapper_count()?,
				assignments.bootstrappers_reputables,
			)),
			RegistrationType::Endorsee =>
				Ok(meetup_index_fn(registration.index - 1, assignments.endorsees)),
			RegistrationType::Newbie =>
				Ok(meetup_index_fn(registration.index - 1, assignments.newbies)),
		}
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
		.unwrap_or_default()
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
		.unwrap_or_default()
		.into_iter()
		.filter(|p| p < &assigned.endorsees)
		.filter_map(|p| self.get_endorsee(community_ceremony, &(p + 1)).ok().flatten());

		let newbies = assignment_fn_inverse(
			meetup_index_zero_based,
			params.newbies,
			meetup_count,
			assigned.newbies,
		)
		.unwrap_or_default()
		.into_iter()
		.filter(|p| p < &assigned.newbies)
		.filter_map(|p| self.get_newbie(community_ceremony, &(p + 1)).ok().flatten());

		Ok(bootstrappers_reputables.chain(endorsees).chain(newbies).collect())
	}

	fn get_meetup_time_offset(&self) -> Result<Option<MeetupTimeOffsetType>> {
		self.get_storage_value(ENCOINTER_CEREMONIES, "MeetupTimeOffset", None)
	}

	fn get_meetup_time(&self, location: Location, one_day: Moment) -> Result<Moment> {
		let attesting_start = self.get_start_of_attesting_phase()?;
		let offset = self.get_meetup_time_offset()?.unwrap_or(0);

		Ok(meetup_time(location, attesting_start, one_day, offset))
	}

	fn get_community_ceremony_stats(
		&self,
		community_ceremony: CommunityCeremony,
	) -> Result<CommunityCeremonyStats> {
		let assignment = self.get_assignments(&community_ceremony)?;
		let assignment_count = self.get_assignment_counts(&community_ceremony)?;
		let mcount = self.get_meetup_count(&community_ceremony)?;

		let mut meetups = vec![];

		// get stats of every meetup
		for m in 1..=mcount {
			let m_location = self.get_meetup_location(&community_ceremony, m)?.unwrap();
			let time = self.get_meetup_time(m_location, ONE_DAY).unwrap_or(0);
			let participants = self.get_meetup_participants(&community_ceremony, m)?;

			let mut registrations = vec![];

			for p in participants.into_iter() {
				let r = self.get_registration(&community_ceremony, &p)?;
				registrations.push((p, r))
			}

			meetups.push(Meetup::new(m, m_location, time, registrations))
		}

		Ok(CommunityCeremonyStats::new(
			community_ceremony,
			assignment,
			assignment_count,
			mcount,
			meetups,
		))
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityCeremonyStats {
	pub community_ceremony: CommunityCeremony,
	pub assignment: Assignment,
	pub assignment_count: AssignmentCount,
	pub meetup_count: MeetupIndexType,
	pub meetups: Vec<Meetup>,
}

impl CommunityCeremonyStats {
	pub fn new(
		community_ceremony: CommunityCeremony,
		assignment: Assignment,
		assignment_count: AssignmentCount,
		meetup_count: MeetupIndexType,
		meetups: Vec<Meetup>,
	) -> Self {
		Self { community_ceremony, assignment, assignment_count, meetup_count, meetups }
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meetup {
	pub index: MeetupIndexType,
	pub location: Location,
	pub time: Moment,
	pub registrations: Vec<(AccountId, Registration)>,
}

impl Meetup {
	pub fn new(
		index: MeetupIndexType,
		location: Location,
		time: Moment,
		registrations: Vec<(AccountId, Registration)>,
	) -> Self {
		Self { index, location, time, registrations }
	}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
	pub index: ParticipantIndexType,
	pub registration_type: RegistrationType,
}

impl Registration {
	pub fn new(index: ParticipantIndexType, registration_type: RegistrationType) -> Self {
		Self { index, registration_type }
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RegistrationType {
	Bootstrapper,
	Reputable,
	Endorsee,
	Newbie,
}
