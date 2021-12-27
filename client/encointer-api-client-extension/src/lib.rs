use sp_core::sr25519;
use substrate_api_client::{rpc::WsRpcClient, ApiClientError};

pub type Result<T> = std::result::Result<T, ApiClientError>;

pub type Api = substrate_api_client::Api<sr25519::Pair, WsRpcClient>;

pub use ceremonies::*;

mod ceremonies;
