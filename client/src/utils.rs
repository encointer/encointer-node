use codec::Encode;
use sp_core::{sr25519, Pair};
use substrate_api_client::{
	compose_extrinsic_offline, rpc::WsRpcClient, Api, UncheckedExtrinsicV4,
};

/// Wrapper around the `compose_extrinsic_offline!` macro to be less verbose.
pub fn offline_xt<C: Encode + Clone>(
	api: &Api<sr25519::Pair, WsRpcClient>,
	call: C,
	nonce: u32,
) -> UncheckedExtrinsicV4<C> {
	compose_extrinsic_offline!(
		api.clone().signer.unwrap(),
		call,
		nonce,
		Era::Immortal,
		api.genesis_hash,
		api.genesis_hash,
		api.runtime_version.spec_version,
		api.runtime_version.transaction_version
	)
}
