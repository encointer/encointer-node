use crate::{Api, Result};
use encointer_node_runtime::{AccountId, Hash};
use encointer_primitives::{communities::CommunityIdentifier, scheduler::CeremonyIndexType};
use parity_scale_codec::Encode;
use substrate_api_client::GetStorage;

#[maybe_async::maybe_async(?Send)]
pub trait ReputationRingsApi {
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
		sub_ring_index: u32,
		maybe_at: Option<Hash>,
	) -> Result<Option<Vec<[u8; 32]>>>;

	async fn get_sub_ring_count(
		&self,
		community: CommunityIdentifier,
		ceremony_index: CeremonyIndexType,
		level: u8,
		maybe_at: Option<Hash>,
	) -> Result<u32>;
}

/// Append a key with Blake2_128Concat hashing to a storage key prefix.
fn append_blake2_128concat(prefix: &mut Vec<u8>, key: &[u8]) {
	prefix.extend_from_slice(&sp_core::blake2_128(key));
	prefix.extend_from_slice(key);
}

#[maybe_async::maybe_async(?Send)]
impl ReputationRingsApi for Api {
	async fn get_bandersnatch_key(
		&self,
		account: &AccountId,
		maybe_at: Option<Hash>,
	) -> Result<Option<[u8; 32]>> {
		self.get_storage_map("EncointerReputationRings", "BandersnatchKeys", account, maybe_at)
			.await
	}

	async fn get_ring_members(
		&self,
		community: CommunityIdentifier,
		ceremony_index: CeremonyIndexType,
		level: u8,
		sub_ring_index: u32,
		maybe_at: Option<Hash>,
	) -> Result<Option<Vec<[u8; 32]>>> {
		// RingMembers is a StorageNMap with 4 Blake2_128Concat keys.
		let mut key = self
			.get_storage_map_key_prefix("EncointerReputationRings", "RingMembers")
			.await?;

		append_blake2_128concat(&mut key.0, &community.encode());
		append_blake2_128concat(&mut key.0, &ceremony_index.encode());
		append_blake2_128concat(&mut key.0, &level.encode());
		append_blake2_128concat(&mut key.0, &sub_ring_index.encode());

		self.get_storage_by_key(key, maybe_at).await
	}

	async fn get_sub_ring_count(
		&self,
		community: CommunityIdentifier,
		ceremony_index: CeremonyIndexType,
		level: u8,
		maybe_at: Option<Hash>,
	) -> Result<u32> {
		// SubRingCount is a StorageNMap with 3 Blake2_128Concat keys.
		let mut key = self
			.get_storage_map_key_prefix("EncointerReputationRings", "SubRingCount")
			.await?;

		append_blake2_128concat(&mut key.0, &community.encode());
		append_blake2_128concat(&mut key.0, &ceremony_index.encode());
		append_blake2_128concat(&mut key.0, &level.encode());

		Ok(self.get_storage_by_key(key, maybe_at).await?.unwrap_or(0))
	}
}
