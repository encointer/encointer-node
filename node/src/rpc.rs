//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use encointer_node_notee_runtime::{opaque::Block, AccountId, Balance, BlockNumber, Index, Moment};
use jsonrpsee::RpcModule;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

pub use sc_rpc_api::DenyUnsafe;

/// Full client dependencies.
///
/// Note: `backend` and `offchain_indexing_enabled` are encointer customizations.
pub struct FullDeps<C, P, Backend> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// backend instance used to access offchain storage
	pub backend: Arc<Backend>,
	/// whether offchain-indexing is enabled
	pub offchain_indexing_enabled: bool,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, TBackend>(
	deps: FullDeps<C, P, TBackend>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BlockBuilder<Block>,
	C::Api: pallet_encointer_ceremonies_rpc_runtime_api::CeremoniesApi<Block, AccountId, Moment>,
	C::Api:
		pallet_encointer_communities_rpc_runtime_api::CommunitiesApi<Block, AccountId, BlockNumber>,
	C::Api: pallet_encointer_bazaar_rpc_runtime_api::BazaarApi<Block, AccountId>,
	P: TransactionPool + 'static,
	TBackend: sc_client_api::Backend<Block>,
	<TBackend as sc_client_api::Backend<Block>>::OffchainStorage: 'static,
{
	use pallet_encointer_bazaar_rpc::{BazaarApiServer, BazaarRpc};
	use pallet_encointer_ceremonies_rpc::{CeremoniesApiServer, CeremoniesRpc};
	use pallet_encointer_communities_rpc::{CommunitiesApiServer, CommunitiesRpc};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { client, pool, backend, offchain_indexing_enabled, deny_unsafe } = deps;

	module.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	module.merge(BazaarRpc::new(client.clone(), deny_unsafe).into_rpc())?;

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `module.merge(YourRpcTrait::into_rpc(YourRpcStruct::new(ReferenceToClient, ...)))?;`

	match backend.offchain_storage() {
		Some(storage) => {
			module.merge(
				CommunitiesRpc::new(
					client.clone(),
					storage.clone(),
					offchain_indexing_enabled,
					deny_unsafe,
				)
				.into_rpc(),
			)?;

			module.merge(
				CeremoniesRpc::new(client.clone(), deny_unsafe, storage, offchain_indexing_enabled)
					.into_rpc(),
			)?;
		},
		None => log::warn!(
			"Offchain caching disabled, due to lack of offchain storage support in backend. \n 
			Will not initialize custom RPCs for 'CommunitiesApi' and 'CeremoniesApi'"
		),
	};

	Ok(module)
}
