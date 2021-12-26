use encointer_primitives::{
	ceremonies::{CommunityCeremony, MeetupIndexType},
	communities::Location,
};
use substrate_api_client::{AccountId, ApiClientError};

pub type Result<T> = std::result::Result<T, ApiClientError>;

pub trait CeremoniesApi {
	fn get_meetup_index(
		community_ceremony: CommunityCeremony,
		account_id: AccountId,
	) -> Result<Option<MeetupIndexType>>;

	fn get_meetup_participants(
		community_ceremony: CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Vec<AccountId>>;

	fn get_meetup_location(
		community_ceremony: CommunityCeremony,
		meetup_index: MeetupIndexType,
	) -> Result<Option<Location>>;
}
