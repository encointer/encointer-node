use crate::{cli_args::EncointerArgsExtractor, utils::get_chain_api};
use clap::ArgMatches;
use encointer_api_client_extension::{
	CeremoniesApi, CommunitiesApi, ReputationCommitmentsApi, SchedulerApi,
};
use encointer_node_notee_runtime::{AccountId, Hash};
use encointer_primitives::{
	ceremonies::CeremonyIndexType,
	democracy::ProposalIdType,
	reputation_commitments::{DescriptorType, PurposeIdType},
};
use log::{debug, error};
use parity_scale_codec::{Decode, Encode};
use sp_core::crypto::Ss58Codec;
use substrate_api_client::GetStorage;

pub fn list_commitments(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();
		let cid = api.verify_cid(matches.cid_arg().unwrap(), None).await;
		let maybe_purpose_id = matches.purpose_id_arg();
		let cindex = api.get_ceremony_index(None).await;
		if let Ok((reputation_lifetime, max_purpose_id)) = tokio::try_join!(
			api.get_reputation_lifetime(maybe_at),
			api.get_current_purpose_id(maybe_at)
		) {
			let relevant_cindexes = cindex.saturating_sub(reputation_lifetime)..=cindex;
			debug!("relevant ceremony indexes: {:?}", &relevant_cindexes);
			let pids = match maybe_purpose_id {
				Some(pid) => pid..=pid,
				_ => 0..=max_purpose_id.unwrap_or(0),
			};
			debug!("scanning for purpose_id's: {:?}", pids);
			for purpose_id in pids {
				for c in relevant_cindexes.clone() {
					let mut key_prefix = api
						.get_storage_double_map_key_prefix(
							"EncointerReputationCommitments",
							"Commitments",
							(cid, c),
						)
						.await
						.unwrap();

					// thanks to Identity hashing we can get all accounts for one specific PurposeId and community_ceremony
					key_prefix.0.append(&mut purpose_id.encode());

					let max_keys = 1000;
					let storage_keys = api
						.get_storage_keys_paged(Some(key_prefix), max_keys, None, maybe_at)
						.await
						.unwrap();
					if storage_keys.len() == max_keys as usize {
						error!("results can be wrong because max keys reached for query")
					}
					for storage_key in storage_keys.iter() {
						let maybe_commitment: Option<Option<Hash>> =
							api.get_storage_by_key(storage_key.clone(), maybe_at).await.unwrap();
						if let Some(maybe_hash) = maybe_commitment {
							let account = AccountId::decode(
								&mut storage_key.0[storage_key.0.len() - 32..].as_ref(),
							)
							.unwrap();
							if let Some(hash) = maybe_hash {
								println!(
									"{cid}, {c}, {purpose_id}, {}, {}",
									account.to_ss58check(),
									hash
								);
							} else {
								println!(
									"{cid}, {c}, {purpose_id}, {}, None",
									account.to_ss58check()
								);
							}
						}
					}
				}
			}
		}
		Ok(())
	})
	.into()
}

pub fn list_purposes(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();
		let key_prefix = api
			.get_storage_map_key_prefix("EncointerReputationCommitments", "Purposes")
			.await
			.unwrap();

		let max_keys = 1000;
		let storage_keys = api
			.get_storage_keys_paged(Some(key_prefix), max_keys, None, maybe_at)
			.await
			.unwrap();
		if storage_keys.len() == max_keys as usize {
			error!("results can be wrong because max keys reached for query")
		}
		for storage_key in storage_keys.iter() {
			let maybe_purpose: Option<DescriptorType> =
				api.get_storage_by_key(storage_key.clone(), maybe_at).await.unwrap();
			if let Some(descriptor) = maybe_purpose {
				let purpose_id =
					PurposeIdType::decode(&mut storage_key.0[storage_key.0.len() - 8..].as_ref())
						.unwrap();
				println!("{purpose_id}: {}", String::from_utf8_lossy(descriptor.as_ref()));
			}
		}
		Ok(())
	})
	.into()
}
