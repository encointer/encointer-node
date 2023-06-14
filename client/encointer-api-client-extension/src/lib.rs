use extrinsic_params::CommunityCurrencyTipExtrinsicParams;
use substrate_api_client::{
	ac_primitives::{Config, ExtrinsicSigner, SubstrateKitchensinkConfig, WithExtrinsicParams},
	rpc::WsRpcClient,
};

pub use encointer_node_notee_runtime::Runtime;
pub use substrate_api_client::{api::error::Error as ApiClientError, Result};

pub type EncointerConfig = WithExtrinsicParams<
	SubstrateKitchensinkConfig,
	CommunityCurrencyTipExtrinsicParams<SubstrateKitchensinkConfig>,
>;

pub type Api = substrate_api_client::Api<EncointerConfig, WsRpcClient>;

pub type ParentchainExtrinsicSigner = ExtrinsicSigner<SubstrateKitchensinkConfig>;
pub type ExtrinsicAddress = <EncointerConfig as Config>::Address;

pub use ceremonies::*;
pub use communities::*;
pub use extrinsic_params::*;
pub use scheduler::*;

mod ceremonies;
mod communities;
mod extrinsic_params;
mod scheduler;
