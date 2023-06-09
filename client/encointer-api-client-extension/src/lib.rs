use sp_core::sr25519;
use sp_runtime::MultiSignature;
use substrate_api_client::{rpc::WsRpcClient, StaticExtrinsicSigner};

pub use substrate_api_client::{api::error::Error as ApiClientError, Result};

pub use encointer_node_notee_runtime::Runtime;

pub type ParentchainExtrinsicSigner = StaticExtrinsicSigner<sp_core::sr25519::Pair, MultiSignature>;

pub type Api = substrate_api_client::Api<
	ParentchainExtrinsicSigner,
	WsRpcClient,
	extrinsic_params::CommunityCurrencyTipExtrinsicParams,
	Runtime,
>;

pub use ceremonies::*;
pub use communities::*;
pub use extrinsic_params::*;
pub use scheduler::*;

mod ceremonies;
mod communities;
mod extrinsic_params;
mod scheduler;
