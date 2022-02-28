use codec::Encode;
use encointer_primitives::scheduler::CeremonyIndexType;
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

/// Handles the potential case of a negative ceremony index CLI.
///
/// If negative: returns the `current_ceremony_index` - `ceremony_index`
/// If positive: returns `ceremony_index`
///
/// Panics when `ceremony_index` == 0, or when effective index would be negative.
///
pub fn into_effective_cindex(
	ceremony_index: i32,
	current_ceremony_index: CeremonyIndexType,
) -> CeremonyIndexType {
	match ceremony_index {
		i32::MIN..=-1 => current_ceremony_index - ceremony_index.abs() as u32,
		1..=i32::MAX => ceremony_index as CeremonyIndexType,
		0 => panic!("Zero not allowed as ceremony index"),
	}
}
