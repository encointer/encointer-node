use crate::{Api, Result};
use encointer_node_notee_runtime::{AccountId, Hash};
use encointer_primitives::{communities::CommunityIdentifier, scheduler::CeremonyIndexType};
use parity_scale_codec::Encode;
use substrate_api_client::GetStorage;

#[maybe_async::maybe_async(?Send)]
pub trait ReputationRingApi {
	async fn get_bandersnatch_key(
		&self,
		account: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Option<[u8; 32]>>;

	async fn get_ring_members(
		&self,
		community: CommunityIdentifier,
		ceremony_index: CeremonyIndexType,
		level: u8,
		maybe_at: Option<Hash>,
	) -> Result<Option<Vec<[u8; 32]>>>;
}

/// Append a key with Blake2_128Concat hashing to a storage key prefix.
fn append_blake2_128concat(prefix: &mut Vec<u8>, key: &[u8]) {
	prefix.extend_from_slice(&sp_core::blake2_128(key));
	prefix.extend_from_slice(key);
}

#[maybe_async::maybe_async(?Send)]
impl ReputationRingApi for Api {
	async fn get_bandersnatch_key(
		&self,
		account: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Option<[u8; 32]>> {
		self.get_storage_map("EncointerReputationRing", "BandersnatchKeys", account, maybe_at)
			.await
	}

	async fn get_ring_members(
		&self,
		community: CommunityIdentifier,
		ceremony_index: CeremonyIndexType,
		level: u8,
		maybe_at: Option<Hash>,
	) -> Result<Option<Vec<[u8; 32]>>> {
		// RingMembers is a StorageNMap with 3 Blake2_128Concat keys.
		// Construct the full storage key manually.
		let mut key = self
			.get_storage_map_key_prefix("EncointerReputationRing", "RingMembers")
			.await?;

		append_blake2_128concat(&mut key.0, &community.encode());
		append_blake2_128concat(&mut key.0, &ceremony_index.encode());
		append_blake2_128concat(&mut key.0, &level.encode());

		self.get_storage_by_key(key, maybe_at).await
	}
}
