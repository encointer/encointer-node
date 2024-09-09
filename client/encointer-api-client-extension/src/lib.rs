use substrate_api_client::{
	ac_primitives::{AssetRuntimeConfig, Config, ExtrinsicSigner, WithExtrinsicParams},
	rpc::JsonrpseeClient,
};

pub use encointer_node_notee_runtime::Runtime;
pub use substrate_api_client::{api::error::Error as ApiClientError, Result};

pub type EncointerConfig = WithExtrinsicParams<
	AssetRuntimeConfig,
	CommunityCurrencyTipExtrinsicParams<AssetRuntimeConfig>,
>;

pub type Api = substrate_api_client::Api<EncointerConfig, JsonrpseeClient>;

pub type ParentchainExtrinsicSigner = ExtrinsicSigner<EncointerConfig>;
pub type ExtrinsicAddress = <EncointerConfig as Config>::Address;

pub use bazaar::*;
pub use ceremonies::*;
pub use communities::*;
pub use democracy::*;
pub use extrinsic_params::*;
pub use reputation_commitments::*;
pub use scheduler::*;
pub use treasuries::*;

mod bazaar;
mod ceremonies;
mod communities;
mod democracy;
mod extrinsic_params;
mod reputation_commitments;
mod scheduler;
mod treasuries;
