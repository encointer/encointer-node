use encointer_ceremonies_assignment::assignment_fn_inverse;
use encointer_primitives::{
	ceremonies::{
		Assignment, AssignmentCount, CommunityCeremony, MeetupIndexType, ParticipantIndexType,
	},
	communities::Location,
};
use sp_core::sr25519;
use substrate_api_client::{rpc::WsRpcClient, AccountId, ApiClientError};

const ENCOINTER_CEREMONIES: &'static str = "EncointerCeremonies";

pub type Result<T> = std::result::Result<T, ApiClientError>;

pub type Api = substrate_api_client::Api<sr25519::Pair, WsRpcClient>;

pub trait CeremoniesApi {
	fn get_assignments(&self, community_ceremony: &CommunityCeremony)
		-> Result<Option<Assignment>>;
	fn get_assignment_counts(
		&self,
		community_ceremony: &CommunityCeremony,
	) -> Result<Option<AssignmentCount>>;

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
		meetup_index: &MeetupIndexType,
	) -> Result<Option<Location>>;

	fn get_meetup_participants(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Vec<AccountId>>;
}

impl CeremoniesApi for Api {
	fn get_assignments(
		&self,
		community_ceremony: &CommunityCeremony,
	) -> Result<Option<Assignment>> {
		self.get_storage_map(ENCOINTER_CEREMONIES, "Assignments", community_ceremony, None)
	}

	fn get_assignment_counts(
		&self,
		community_ceremony: &CommunityCeremony,
	) -> Result<Option<AssignmentCount>> {
		self.get_storage_map(ENCOINTER_CEREMONIES, "AssignmentCounts", community_ceremony, None)
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
		todo!()
	}

	fn get_meetup_location(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: &MeetupIndexType,
	) -> Result<Option<Location>> {
		todo!()
	}

	fn get_meetup_participants(
		&self,
		community_ceremony: &CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Vec<AccountId>> {
		let mut participants = vec![];
		let meetup_index_zero_based = meetup_index - 1;

		let meetup_count = self.get_meetup_count(community_ceremony)?;

		if meetup_index_zero_based > meetup_count {
			return Err(ApiClientError::Other(
				format!("Invalid meetup index > meetup count: {}, {}", meetup_index, meetup_count)
					.into(),
			))
		}

		let params = self
			.get_assignments(community_ceremony)?
			.ok_or_else(|| ApiClientError::Other("Assignments not found".into()))?;

		let assigned = self
			.get_assignment_counts(community_ceremony)?
			.ok_or_else(|| ApiClientError::Other("AssignmentCounts not found".into()))?;

		let bootstrappers_reputables = assignment_fn_inverse(
			meetup_index,
			params.bootstrappers_reputables,
			meetup_count,
			assigned.bootstrappers + assigned.reputables,
		)
		.into_iter()
		.map(|p_index| {
			get_bootstrapper_or_reputable(self, community_ceremony, p_index, &assigned).ok()?
		})
		.collect::<Vec<_>>();

		Ok(participants)
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
