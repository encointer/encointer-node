use sp_core::sr25519;
use substrate_api_client::rpc::WsRpcClient;

pub use substrate_api_client::{ApiClientError, ApiResult as Result};

pub type Api = substrate_api_client::Api<sr25519::Pair, WsRpcClient>;

pub use ceremonies::*;
pub use communities::*;

mod ceremonies;
mod communities;
