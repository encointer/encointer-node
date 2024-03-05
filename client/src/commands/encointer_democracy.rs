use crate::cli_args::EncointerArgsExtractor;
use crate::commands::encointer_core::set_api_extrisic_params_builder;
use crate::commands::encointer_core::verify_cid;
use crate::commands::encointer_scheduler::get_ceremony_index;
use crate::utils::ensure_payment;
use crate::utils::get_chain_api;
use crate::utils::keys::get_pair_from_str;
use clap::ArgMatches;
use encointer_api_client_extension::{EncointerXt, ParentchainExtrinsicSigner};
use encointer_node_notee_runtime::BlockNumber;
use encointer_primitives::ceremonies::{CeremonyIndexType, CommunityCeremony};
use encointer_primitives::democracy::ProposalAction;
use encointer_primitives::democracy::{Proposal, ProposalIdType, ReputationVec, Vote};
use log::error;
use parity_scale_codec::{Decode, Encode};
use sp_core::{sr25519 as sr25519_core, ConstU32};
use substrate_api_client::ac_compose_macros::compose_extrinsic;
use substrate_api_client::GetStorage;
use substrate_api_client::SubmitAndWatch;
use substrate_api_client::XtStatus;

pub fn submit_set_inactivity_timeout_proposal(
	_args: &str,
	matches: &ArgMatches<'_>,
) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let inactivity_timeout = matches.inactivity_timeout_arg().unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"submit_proposal",
			ProposalAction::SetInactivityTimeout(inactivity_timeout)
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal Submitted: Set inactivity timeout to {inactivity_timeout:?}");
		Ok(())
	})
	.into()
}
pub fn list_proposals(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let at_block = matches.at_block_arg();
		let key_prefix =
			api.get_storage_map_key_prefix("EncointerDemocracy", "Proposals").await.unwrap();
		let max_keys = 1000;
		let storage_keys = api
			.get_storage_keys_paged(Some(key_prefix), max_keys, None, at_block)
			.await
			.unwrap();
		if storage_keys.len() == max_keys as usize {
			error!("results can be wrong because max keys reached for query")
		}
		for storage_key in storage_keys.iter() {
			let key_postfix = storage_key.as_ref();
			let proposal_id =
				ProposalIdType::decode(&mut key_postfix[key_postfix.len() - 16..].as_ref())
					.unwrap();
			let proposal: Proposal<BlockNumber> =
				api.get_storage_by_key(storage_key.clone(), at_block).await.unwrap().unwrap();
			println!("id: {}", proposal_id);
			println!("action: {:?}", proposal.action);
			println!("start block: {}", proposal.start);
			println!("start cindex: {}", proposal.start_cindex);
			println!("state: {:?}", proposal.state);
			println!("");
		}
		Ok(())
	})
	.into()
}
pub fn vote(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let proposal_id = matches.proposal_id_arg().unwrap();
		let vote_raw = matches.vote_arg().unwrap();
		let vote = match vote_raw {
			"aye" => Vote::Aye,
			"nay" => Vote::Nay,
			&_ => panic!("invalid vote"),
		};
		let reputation_vec: Vec<CommunityCeremony> = futures::future::join_all(
			matches
				.reputation_vec_arg()
				.ok_or(clap::Error::with_description(
					"missing reputation-vec argument",
					clap::ErrorKind::MissingRequiredArgument,
				))?
				.into_iter()
				.map(|rep| {
					let api_local = api.clone();
					async move {
						let cc: Vec<_> = rep.split("_").collect();
						(
							verify_cid(&api_local, cc[0], None).await,
							cc[1].parse::<CeremonyIndexType>().unwrap(),
						)
					}
				}),
		)
		.await;
		let reputation_bvec = ReputationVec::<ConstU32<1024>>::try_from(reputation_vec);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"vote",
			proposal_id,
			vote,
			reputation_bvec
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Vote submitted: {vote_raw:?} for proposal {proposal_id:?}");
		Ok(())
	})
	.into()
}

pub fn update_proposal_state(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let proposal_id = matches.proposal_id_arg().unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerDemocracy", "update_proposal_state", proposal_id)
				.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal state updated for proposal {proposal_id:?}");
		Ok(())
	})
	.into()
}
