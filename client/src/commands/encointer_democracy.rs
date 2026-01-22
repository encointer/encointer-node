use crate::cli_args::EncointerArgsExtractor;

use crate::{
	community_spec::demurrage_per_block_from_halving_blocks,
	utils::{
		ensure_payment, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
	types::XcmLocation,
};
use chrono::{prelude::*, Utc};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, Api, CeremoniesApi, CommunitiesApi, DemocracyApi, EncointerXt,
	Moment, ParentchainExtrinsicSigner, SchedulerApi,
};
use encointer_node_notee_runtime::{AccountId, Balance, Hash};
use encointer_primitives::{
	ceremonies::{CeremonyIndexType, CommunityCeremony, ReputationCountType},
	common::{FromStr, PalletString},
	communities::CommunityIdentifier,
	democracy::{
		ProposalAccessPolicy, ProposalIdType, ProposalState,
		ReputationVec, Vote,
	},
};
use log::{debug, error};
use parity_scale_codec::{Decode, Encode};
use sp_core::{sr25519 as sr25519_core, ConstU32};
use substrate_api_client::{
	ac_compose_macros::compose_extrinsic, GetStorage, SubmitAndWatch, XtStatus,
};

// * All proposal variants that don't have anything to do with assets will decode fine on either chain.
// * We only get an issue if we want to decode the `SpendAsset` and `IssueSwapAssetOption` with the wrong type.
//
// We currently just assume that the solochain will never have any asset proposals. To properly handle this, we
// need to add a flag to the CLI to configure which type to use, i.e., `--is-para`.
pub type Proposal = encointer_primitives::democracy::Proposal<Moment, AccountId, Balance, XcmLocation>;
pub type ProposalAction = encointer_primitives::democracy::ProposalAction<AccountId, Balance, Moment, XcmLocation>;

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

pub fn submit_update_nominal_income_proposal(
	_args: &str,
	matches: &ArgMatches<'_>,
) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;
		let new_income = matches.nominal_income_arg().unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"submit_proposal",
			ProposalAction::UpdateNominalIncome(cid, new_income)
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal Submitted: Update nominal income for cid {cid} to {new_income}");
		Ok(())
	})
	.into()
}

pub fn submit_update_demurrage_proposal(
	_args: &str,
	matches: &ArgMatches<'_>,
) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;
		let new_demurrage_halving_blocks = matches.demurrage_halving_blocks_arg().unwrap();
		let new_demurrage_per_block =
			demurrage_per_block_from_halving_blocks(new_demurrage_halving_blocks);
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"submit_proposal",
			ProposalAction::UpdateDemurrage(
				cid,
				new_demurrage_per_block
			)
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal Submitted: Update demurrage for cid {cid} to {new_demurrage_per_block}");
		Ok(())
	})
	.into()
}
pub fn submit_petition(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let maybecid = if let Some(cid) = matches.cid_arg() {
			Some(api.verify_cid(cid, None).await)
		} else {
			None
		};
		let demand_str = matches.value_of("demand").unwrap();
		let demand = PalletString::from_str(demand_str)
			.expect("Petition demand too long. must be < 256 chars");
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"submit_proposal",
			ProposalAction::Petition(maybecid, demand.clone())
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal Submitted: Petition for cid {maybecid:?} demanding: {demand_str}");
		Ok(())
	})
	.into()
}
pub fn submit_spend_native_proposal(
	_args: &str,
	matches: &ArgMatches<'_>,
) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let maybecid = if let Some(cid) = matches.cid_arg() {
			Some(api.verify_cid(cid, None).await)
		} else {
			None
		};
		let arg_to = matches.value_of("to").unwrap();
		let to = get_accountid_from_str(arg_to);
		let amount = matches
			.value_of("amount")
			.unwrap()
			.parse::<u128>()
			.expect("amount can be converted to u128");
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"submit_proposal",
			ProposalAction::SpendNative(maybecid, to.clone(), amount)
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal Submitted: Spend Native for cid {maybecid:?} to {to}, amount {amount}");
		Ok(())
	})
	.into()
}
pub fn list_proposals(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();
		let key_prefix =
			api.get_storage_map_key_prefix("EncointerDemocracy", "Proposals").await.unwrap();
		let max_keys = 1000;
		let storage_keys = api
			.get_storage_keys_paged(Some(key_prefix), max_keys, None, maybe_at)
			.await
			.unwrap();
		if storage_keys.len() == max_keys as usize {
			error!("results can be wrong because max keys reached for query")
		}
		let confirmation_period = api.get_confirmation_period().await.unwrap();
		let proposal_lifetime = api.get_proposal_lifetime().await.unwrap();
		let min_turnout_permill = api.get_min_turnout().await.unwrap();
		println!("📜 Number of proposals: {}, global config: proposal lifetime: {:?}, confirmation period: {:?}, min turnout: {:.3}%", storage_keys.len(), proposal_lifetime, confirmation_period, min_turnout_permill as f64 / 10f64);
		let mut proposals: Vec<(ProposalIdType, Proposal)> = Vec::new();
		for storage_key in storage_keys.iter() {
			debug!("storage_key: 0x{}", hex::encode(storage_key));
			let key_postfix = storage_key.as_ref();
			let proposal_id =
				ProposalIdType::decode(&mut key_postfix[key_postfix.len() - 16..].as_ref())
					.unwrap();
			debug!("proposalid: {:?}", proposal_id);
			let proposal: Proposal =
				api.get_storage_by_key(storage_key.clone(), maybe_at).await.unwrap().unwrap();
			if !matches.all_flag() && proposal.state.has_failed() {
				continue
			}
			proposals.push((proposal_id, proposal));
		};
		proposals.sort_by_key(|p| p.0);
		for (proposal_id, proposal) in proposals.iter() {
			let start = DateTime::<Utc>::from_timestamp_millis(
				TryInto::<i64>::try_into(proposal.start).unwrap(),
			)
			.unwrap();
			// let electorate = get_relevant_electorate(
			// 	&api,
			// 	proposal.action.clone().get_access_policy(),
			// 	maybe_at,
			// )
			// .await;
			let maybe_confirming_since = match proposal.state {
				ProposalState::Confirming { since } => Some(
					DateTime::<Utc>::from_timestamp_millis(
						TryInto::<i64>::try_into(since).unwrap(),
					)
					.unwrap(),
				),
				_ => None,
			};
			let electorate = get_relevant_electorate(
				&api,
				proposal.start_cindex,
				proposal.action.clone().get_access_policy(),
				maybe_at,
			)
			.await;
			let tally = api.get_tally(*proposal_id, maybe_at).await.unwrap().unwrap_or_default();
			let purpose_id = api.get_purpose_id(*proposal_id, maybe_at).await.unwrap().unwrap();
			println!(
				"Proposal id: {} (reputation commitment purpose id: {})",
				*proposal_id, purpose_id
			);
			let proposal_str = match &proposal.action {
				ProposalAction::SetInactivityTimeout(timeout) =>
					format!("Set inactivity timeout to {timeout}"),
				ProposalAction::UpdateNominalIncome(cid, income) =>
					format!("Update nominal income for {cid} to {income}"),
				ProposalAction::Petition(maybecid, demand) =>
					format!("Petition for {} demanding: {}", cid_or_global(maybecid), String::from_utf8_lossy(demand)),
				ProposalAction::SpendNative(maybecid, to, amount) =>
					format!("Spend Native from {} treasury to {to}, amount {amount}", cid_or_global(maybecid)),
				_ => format!("{:?}", proposal.action),
			};
			println!("🛠 action: {:?}", proposal_str);
			println!("▶️ started at: {}", start.format("%Y-%m-%d %H:%M:%S %Z").to_string());
			println!(
				"🏁 ends after: {}",
				(start + proposal_lifetime.clone()).format("%Y-%m-%d %H:%M:%S %Z").to_string()
			);
			println!("🔄 start cindex: {}", proposal.start_cindex);
			println!("👥 electorate: {electorate}");
			println!(
				"🗳 turnout: {} votes = {:.3}% of electorate (turnout threshold {} votes = {:.3}%)",
				tally.turnout,
				100f64 * tally.turnout as f64 / electorate as f64,
				min_turnout_permill as f64 * electorate as f64 / 1000f64,
				min_turnout_permill as f64 / 10f64
			);
			println!(
				"🗳 approval: {} votes = {:.3}% Aye (AQB approval threshold: {:.3}%)",
				tally.ayes,
				100f64 * tally.ayes as f64 / tally.turnout as f64,
				approval_threshold_percent(electorate, tally.turnout)
			);
			println!("state: {:?}", proposal.state);
			if let Some(since) = maybe_confirming_since {
				println!(
					"👍 confirming since: {} until {}",
					since.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
					(since + confirmation_period).format("%Y-%m-%d %H:%M:%S %Z").to_string()
				)
			}
			println!("");
		}
		Ok(())
	})
	.into()
}

pub fn list_enactment_queue(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();
		let key_prefix = api
			.get_storage_map_key_prefix("EncointerDemocracy", "EnactmentQueue")
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
			let maybe_proposal_id: Option<ProposalIdType> =
				api.get_storage_by_key(storage_key.clone(), maybe_at).await.unwrap();
			if let Some(proposal_id) = maybe_proposal_id {
				println!("{}", proposal_id);
			}
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
							api_local.verify_cid(cc[0], None).await,
							cc[1].parse::<CeremonyIndexType>().unwrap(),
						)
					}
				}),
		)
		.await;
		let reputation_bvec = ReputationVec::<ConstU32<1024>>::try_from(reputation_vec).unwrap();

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
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await;
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

/// count reputation assuming we would start
async fn get_relevant_electorate(
	api: &Api,
	proposal_start_cindex: CeremonyIndexType,
	scope: ProposalAccessPolicy,
	maybe_at: Option<Hash>,
) -> ReputationCountType {
	if let Ok((reputation_lifetime, cycle_duration, proposal_lifetime)) = tokio::try_join!(
		api.get_reputation_lifetime(maybe_at),
		api.get_cycle_duration(maybe_at),
		api.get_proposal_lifetime()
	) {
		let proposal_lifetime_cycles =
			u32::try_from(proposal_lifetime.as_millis().div_ceil(cycle_duration as u128)).unwrap();
		let relevant_cindexes = (proposal_start_cindex
			.saturating_sub(reputation_lifetime)
			.saturating_add(proposal_lifetime_cycles)..=
			proposal_start_cindex.saturating_sub(2u32))
			.collect::<Vec<CeremonyIndexType>>();
		let mut count: ReputationCountType = 0;
		for c in relevant_cindexes {
			count += match scope {
				ProposalAccessPolicy::Community(cid) =>
					api.get_reputation_count((cid, c), maybe_at).await.unwrap_or(0),
				ProposalAccessPolicy::Global =>
					api.get_global_reputation_count(c, maybe_at).await.unwrap_or(0),
			};
		}
		return count;
	} else {
		panic!("couldn't fetch some values")
	}
}

fn approval_threshold_percent(electorate: u128, turnout: u128) -> f64 {
	100f64 / (1f64 + (turnout as f64 / electorate as f64).sqrt())
}

fn cid_or_global(maybecid: &Option<CommunityIdentifier>) -> String {
	match maybecid {
		Some(cid) => format!("{:?}", cid),
		None => "global".into(),
	}
}
