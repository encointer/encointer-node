//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use encointer_node_notee_runtime::{opaque::Block, AccountId, Balance, Index};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_block_builder::BlockBuilder;
pub use sc_rpc_api::DenyUnsafe;
use sp_transaction_pool::TransactionPool;


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
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata> where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BlockBuilder<Block>,
	C::Api: pallet_encointer_communities_rpc_runtime_api::CommunitiesApi<Block>,
	P: TransactionPool + 'static,
	TBackend: sc_client_api::Backend<Block>,
	<TBackend as sc_client_api::Backend<Block>>::OffchainStorage: 'static,
{
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use pallet_encointer_communities_rpc::{CommunitiesApi, Communities};

	let mut io = jsonrpc_core::IoHandler::default();
	let FullDeps {
		client,
		pool,
		backend,
		offchain_indexing_enabled,
		deny_unsafe,
	} = deps;

	io.extend_with(
		SystemApi::to_delegate(FullSystem::new(client.clone(), pool, deny_unsafe))
	);

	io.extend_with(
		TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
	);

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `io.extend_with(YourRpcTrait::to_delegate(YourRpcStruct::new(ReferenceToClient, ...)));`

	match backend.offchain_storage() {
		Some(storage) => io.extend_with(
			CommunitiesApi::to_delegate(Communities::new(client.clone(), storage, offchain_indexing_enabled))
		),
		None => log::warn!("Offchain caching disabled, due to lack of offchain storage support in backend.")
	};

	io
}
