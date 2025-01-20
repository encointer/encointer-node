use crate::{Api, CommunitiesApi, Result, SchedulerApi};
use encointer_ceremonies_assignment::{
	assignment_fn_inverse, meetup_index, meetup_location, meetup_time,
};
use encointer_node_notee_runtime::{CeremonyIndexType, Hash};
use encointer_primitives::{
	ceremonies::{
		Assignment, AssignmentCount, CommunityCeremony, MeetupIndexType, MeetupTimeOffsetType,
		ParticipantIndexType, ReputationCountType, ReputationLifetimeType,
	},
	communities::Location,
};
use futures::stream::{self, StreamExt};
use log::warn;
use serde::{Deserialize, Serialize};
use sp_runtime::AccountId32 as AccountId;
use substrate_api_client::{api::error::Error as ApiClientError, GetStorage};
pub type Moment = u64;

pub const ENCOINTER_CEREMONIES: &str = "EncointerCeremonies";

// same as in runtime, but we did not want to import the runtime here.
pub const ONE_DAY: Moment = 86_400_000;

#[maybe_async::maybe_async(?Send)]
pub trait CeremoniesApi {
	async fn get_assignments(
		&self,
		community_ceremony: &CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<Assignment>;
	async fn get_assignment_counts(
		&self,
		community_ceremony: &CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<AssignmentCount>;

	async fn get_bootstrapper(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>>;

	async fn get_reputable(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>>;

	async fn get_endorsee(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>>;

	async fn get_newbie(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>>;

	async fn get_registration(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Registration>;

	async fn get_meetup_count(
		&self,
		community_ceremony: &CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<MeetupIndexType>;

	async fn get_meetup_index(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Option<MeetupIndexType>>;

	async fn get_meetup_location(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<Location>>;

	async fn get_meetup_participants(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Vec<AccountId>>;

	async fn get_meetup_time_offset(
		&self,
		maybe_at: Option<Hash>,
	) -> Result<Option<MeetupTimeOffsetType>>;

	async fn get_meetup_time(
		&self,
		location: Location,
		one_day: Moment,
		maybe_at: Option<Hash>,
	) -> Result<Moment>;

	async fn get_community_ceremony_stats(
		&self,
		community_ceremony: CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<CommunityCeremonyStats>;

	async fn get_attestees(
		&self,
		community_ceremony: CommunityCeremony,
		participant_index: ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Vec<AccountId>>;

	async fn get_meetup_participant_count_vote(
		&self,
		community_ceremony: CommunityCeremony,
		account_id: AccountId,
		maybe_at: Option<Hash>,
	) -> Result<u32>;
	async fn get_global_reputation_count(
		&self,
		cindex: CeremonyIndexType,
		maybe_at: Option<Hash>,
	) -> Result<ReputationCountType>;
	async fn get_reputation_count(
		&self,
		community_ceremony: CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<ReputationCountType>;
	async fn get_reputation_lifetime(
		&self,
		maybe_at: Option<Hash>,
	) -> Result<ReputationLifetimeType>;
	async fn get_participant_attestation_index(
		&self,
		key: CommunityCeremony,
		accountid: &encointer_node_notee_runtime::AccountId,
		maybe_at: Option<Hash>,
	) -> Option<ParticipantIndexType>;
	async fn get_attestee_count(
		&self,
		key: CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> ParticipantIndexType;
}

#[maybe_async::maybe_async(?Send)]
impl CeremoniesApi for Api {
	async fn get_assignments(
		&self,
		community_ceremony: &CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<Assignment> {
		self.get_storage_map(ENCOINTER_CEREMONIES, "Assignments", community_ceremony, maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("Assignments don't exist".into()))
	}

	async fn get_assignment_counts(
		&self,
		community_ceremony: &CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<AssignmentCount> {
		self.get_storage_map(ENCOINTER_CEREMONIES, "AssignmentCounts", community_ceremony, maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("AssignmentCounts not found".into()))
	}

	async fn get_bootstrapper(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"BootstrapperRegistry",
			community_ceremony,
			p,
			maybe_at,
		)
		.await
	}

	async fn get_reputable(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"ReputableRegistry",
			community_ceremony,
			p,
			maybe_at,
		)
		.await
	}

	async fn get_endorsee(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"EndorseeRegistry",
			community_ceremony,
			p,
			maybe_at,
		)
		.await
	}

	async fn get_newbie(
		&self,
		community_ceremony: &CommunityCeremony,
		p: &ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<AccountId>> {
		self.get_storage_double_map(
			ENCOINTER_CEREMONIES,
			"NewbieRegistry",
			community_ceremony,
			p,
			maybe_at,
		)
		.await
	}

	async fn get_registration(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Registration> {
		let index_query = |storage_key| async move {
			self.get_storage_double_map(
				ENCOINTER_CEREMONIES,
				storage_key,
				community_ceremony,
				account_id,
				maybe_at,
			)
			.await
		};

		if let Some(p_index) = index_query("BootstrapperIndex").await? {
			return Ok(Registration::new(p_index, RegistrationType::Bootstrapper));
		} else if let Some(p_index) = index_query("ReputableIndex").await? {
			return Ok(Registration::new(p_index, RegistrationType::Reputable));
		} else if let Some(p_index) = index_query("EndorseeIndex").await? {
			return Ok(Registration::new(p_index, RegistrationType::Endorsee));
		} else if let Some(p_index) = index_query("NewbieIndex").await? {
			return Ok(Registration::new(p_index, RegistrationType::Newbie));
		}

		Err(ApiClientError::Other(
			format!("Could not get participant index for {account_id:?}").into(),
		))
	}

	async fn get_meetup_count(
		&self,
		community_ceremony: &CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<MeetupIndexType> {
		Ok(self
			.get_storage_map(ENCOINTER_CEREMONIES, "MeetupCount", community_ceremony, maybe_at)
			.await?
			.unwrap_or(0))
	}

	async fn get_meetup_index(
		&self,
		community_ceremony: &CommunityCeremony,
		account_id: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Option<MeetupIndexType>> {
		let meetup_count = self.get_meetup_count(community_ceremony, maybe_at).await?;

		if meetup_count == 0 {
			warn!("Meetup Count is 0.");
			return Ok(None);
		}

		let assignments = self.get_assignments(community_ceremony, maybe_at).await?;

		// Some helper queries to make below code more readable.
		let bootstrapper_count = || async {
			Ok::<ParticipantIndexType, ApiClientError>(
				self.get_assignment_counts(community_ceremony, maybe_at).await?.bootstrappers,
			)
		};

		let registration = self.get_registration(community_ceremony, account_id, maybe_at).await?;

		let meetup_index_fn =
			|p_index, assignment_params| meetup_index(p_index, assignment_params, meetup_count);

		// Finally get the meetup index

		match registration.registration_type {
			RegistrationType::Bootstrapper => {
				Ok(meetup_index_fn(registration.index - 1, assignments.bootstrappers_reputables))
			},
			RegistrationType::Reputable => Ok(meetup_index_fn(
				registration.index - 1 + bootstrapper_count().await?,
				assignments.bootstrappers_reputables,
			)),
			RegistrationType::Endorsee => {
				Ok(meetup_index_fn(registration.index - 1, assignments.endorsees))
			},
			RegistrationType::Newbie => {
				Ok(meetup_index_fn(registration.index - 1, assignments.newbies))
			},
		}
	}

	async fn get_meetup_location(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Option<Location>> {
		let locations = self.get_locations(community_ceremony.0).await?;
		let location_assignment_params =
			self.get_assignments(community_ceremony, maybe_at).await?.locations;

		Ok(meetup_location(meetup_index, locations, location_assignment_params))
	}

	async fn get_meetup_participants(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Vec<AccountId>> {
		let meetup_index_zero_based = meetup_index - 1;
		let meetup_count = self.get_meetup_count(community_ceremony, maybe_at).await?;

		if meetup_index_zero_based > meetup_count {
			return Err(ApiClientError::Other(
				format!(
					"Invalid meetup index > meetup count: {meetup_index_zero_based}, {meetup_count}"
				)
				.into(),
			));
		}

		let params = self.get_assignments(community_ceremony, maybe_at).await?;
		let assigned = self.get_assignment_counts(community_ceremony, maybe_at).await?;

		let bootstrappers_reputables = stream::iter(
			assignment_fn_inverse(
				meetup_index_zero_based,
				params.bootstrappers_reputables,
				meetup_count,
				assigned.bootstrappers + assigned.reputables,
			)
			.unwrap_or_default(),
		)
		.filter_map(|p_index| async move {
			get_bootstrapper_or_reputable(self, community_ceremony, p_index, &assigned, maybe_at)
				.await
				.ok()
				.flatten()
		});

		let endorsees = stream::iter(
			assignment_fn_inverse(
				meetup_index_zero_based,
				params.endorsees,
				meetup_count,
				assigned.endorsees,
			)
			.unwrap_or_default()
			.into_iter()
			.filter(|p| p < &assigned.endorsees),
		)
		.filter_map(|p| async move {
			self.get_endorsee(community_ceremony, &(p + 1), maybe_at).await.ok().flatten()
		});

		let newbies = stream::iter(
			assignment_fn_inverse(
				meetup_index_zero_based,
				params.newbies,
				meetup_count,
				assigned.newbies,
			)
			.unwrap_or_default()
			.into_iter()
			.filter(|p| p < &assigned.newbies),
		)
		.filter_map(|p| async move {
			self.get_newbie(community_ceremony, &(p + 1), maybe_at).await.ok().flatten()
		});

		Ok(bootstrappers_reputables.chain(endorsees).chain(newbies).collect().await)
	}

	async fn get_meetup_time_offset(
		&self,
		maybe_at: Option<Hash>,
	) -> Result<Option<MeetupTimeOffsetType>> {
		self.get_storage(ENCOINTER_CEREMONIES, "MeetupTimeOffset", maybe_at).await
	}

	async fn get_meetup_time(
		&self,
		location: Location,
		one_day: Moment,
		maybe_at: Option<Hash>,
	) -> Result<Moment> {
		let attesting_start = self.get_start_of_attesting_phase(maybe_at).await?;
		let offset = self.get_meetup_time_offset(maybe_at).await?.unwrap_or(0);

		Ok(meetup_time(location, attesting_start, one_day, offset))
	}

	async fn get_community_ceremony_stats(
		&self,
		community_ceremony: CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<CommunityCeremonyStats> {
		let assignment = self.get_assignments(&community_ceremony, maybe_at).await?;
		let assignment_count = self.get_assignment_counts(&community_ceremony, maybe_at).await?;
		let mcount = self.get_meetup_count(&community_ceremony, maybe_at).await?;

		let mut meetups = vec![];

		// get stats of every meetup
		for m in 1..=mcount {
			let m_location =
				self.get_meetup_location(&community_ceremony, m, maybe_at).await?.unwrap();
			let time = self.get_meetup_time(m_location, ONE_DAY, maybe_at).await.unwrap_or(0);
			let participants =
				self.get_meetup_participants(&community_ceremony, m, maybe_at).await?;

			let mut registrations = vec![];

			for p in participants.into_iter() {
				let r = self.get_registration(&community_ceremony, &p, maybe_at).await?;
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

	async fn get_attestees(
		&self,
		community_ceremony: CommunityCeremony,
		p_index: ParticipantIndexType,
		maybe_at: Option<Hash>,
	) -> Result<Vec<AccountId>> {
		self.get_storage_double_map(
			"EncointerCeremonies",
			"AttestationRegistry",
			community_ceremony,
			p_index,
			maybe_at,
		)
		.await?
		.ok_or_else(|| ApiClientError::Other("Attestees don't exist".into()))
	}

	async fn get_meetup_participant_count_vote(
		&self,
		community_ceremony: CommunityCeremony,
		account_id: AccountId,
		maybe_at: Option<Hash>,
	) -> Result<u32> {
		self.get_storage_double_map(
			"EncointerCeremonies",
			"MeetupParticipantCountVote",
			community_ceremony,
			account_id,
			maybe_at,
		)
		.await?
		.ok_or_else(|| ApiClientError::Other("MeetupParticipantCountVote don't exist".into()))
	}

	async fn get_reputation_count(
		&self,
		community_ceremony: CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> Result<ReputationCountType> {
		self.get_storage_map("EncointerCeremonies", "ReputationCount", community_ceremony, maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("ReputationCount not found".into()))
	}
	async fn get_global_reputation_count(
		&self,
		cindex: CeremonyIndexType,
		maybe_at: Option<Hash>,
	) -> Result<ReputationCountType> {
		self.get_storage_map("EncointerCeremonies", "GlobalReputationCount", cindex, maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("GlobalReputationCount not found".into()))
	}
	async fn get_reputation_lifetime(
		&self,
		maybe_at: Option<Hash>,
	) -> Result<ReputationLifetimeType> {
		self.get_storage("EncointerCeremonies", "ReputationLifetime", maybe_at)
			.await?
			.ok_or_else(|| ApiClientError::Other("ReputationLifetime not found".into()))
	}
	async fn get_attestee_count(
		&self,
		key: CommunityCeremony,
		maybe_at: Option<Hash>,
	) -> ParticipantIndexType {
		self.get_storage_map("EncointerCeremonies", "AttestationCount", key, maybe_at)
			.await
			.unwrap()
			.unwrap_or(0)
	}

	async fn get_participant_attestation_index(
		&self,
		key: CommunityCeremony,
		accountid: &encointer_node_notee_runtime::AccountId,
		maybe_at: Option<Hash>,
	) -> Option<ParticipantIndexType> {
		self.get_storage_double_map(
			"EncointerCeremonies",
			"AttestationIndex",
			key,
			accountid,
			maybe_at,
		)
		.await
		.unwrap()
	}
}

async fn get_bootstrapper_or_reputable(
	api: &Api,
	community_ceremony: &CommunityCeremony,
	p_index: ParticipantIndexType,
	assigned: &AssignmentCount,
	maybe_at: Option<Hash>,
) -> Result<Option<AccountId>> {
	if p_index < assigned.bootstrappers {
		return api.get_bootstrapper(community_ceremony, &(p_index + 1), maybe_at).await;
	} else if p_index < assigned.bootstrappers + assigned.reputables {
		return api
			.get_reputable(community_ceremony, &(p_index - assigned.bootstrappers + 1), maybe_at)
			.await;
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
pub struct AttestationState {
	pub community_ceremony: CommunityCeremony,
	pub meetup_index: MeetupIndexType,
	pub vote: u32,
	pub attestation_index: u64,
	pub attestor: AccountId,
	pub attestees: Vec<AccountId>,
}

impl AttestationState {
	pub fn new(
		community_ceremony: CommunityCeremony,
		meetup_index: MeetupIndexType,
		vote: u32,
		attestation_index: u64,
		attestor: AccountId,
		attestees: Vec<AccountId>,
	) -> Self {
		Self { community_ceremony, meetup_index, vote, attestation_index, attestor, attestees }
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
