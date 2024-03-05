use crate::{
	cli_args::EncointerArgsExtractor,
	commands::{
		encointer_communities::get_cid_names,
		encointer_core::{set_api_extrisic_params_builder, verify_cid},
		encointer_scheduler::get_ceremony_index,
	},
	exit_code,
	utils::{
		collective_propose_call, contains_sudo_pallet, ensure_payment, get_chain_api,
		get_councillors, into_effective_cindex,
		keys::{get_accountid_from_str, get_pair_from_str},
		print_raw_call, send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	Api, ApiClientError, CeremoniesApi, EncointerXt, ParentchainExtrinsicSigner, SchedulerApi,
	ENCOINTER_CEREMONIES,
};
use encointer_node_notee_runtime::{AccountId, Hash, Moment, Signature, ONE_DAY};
use encointer_primitives::{
	ceremonies::{
		CeremonyIndexType, ClaimOfAttendance, CommunityCeremony, CommunityReputation,
		MeetupIndexType, ParticipantIndexType, ProofOfAttendance, Reputation,
		ReputationLifetimeType,
	},
	communities::CommunityIdentifier,
	scheduler::CeremonyPhaseType,
};
use log::{debug, error, info};
use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::sr25519;
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_keyring::AccountKeyring;
use sp_runtime::MultiSignature;
use std::collections::HashMap;
use substrate_api_client::{
	ac_compose_macros::{compose_call, compose_extrinsic, rpc_params},
	ac_primitives::{Bytes, SignExtrinsic},
	rpc::Request,
	GetStorage, SubmitAndWatch, XtStatus,
};

pub fn list_participants(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let cindex = matches.ceremony_index_arg().map_or_else(
			|| current_ceremony_index,
			|ci| into_effective_cindex(ci, current_ceremony_index),
		);

		println!("listing participants for cid {cid} and ceremony nr {cindex}");

		let counts = vec!["BootstrapperCount", "ReputableCount", "EndorseeCount", "NewbieCount"];

		let registries =
			vec!["BootstrapperRegistry", "ReputableRegistry", "EndorseeRegistry", "NewbieRegistry"];

		let mut num_participants: Vec<u64> = vec![0, 0, 0, 0];
		for i in 0..registries.len() {
			println!("Querying {}", registries[i]);

			let count: ParticipantIndexType = api
				.get_storage_map(ENCOINTER_CEREMONIES, counts[i], (cid, cindex), None)
				.await
				.unwrap()
				.unwrap_or(0);
			println!("number of participants assigned:  {count}");
			num_participants[i] = count;
			for p_index in 1..count + 1 {
				let accountid: AccountId = api
					.get_storage_double_map(
						ENCOINTER_CEREMONIES,
						registries[i],
						(cid, cindex),
						p_index,
						None,
					)
					.await
					.unwrap()
					.unwrap();
				println!("{}[{}, {}] = {}", registries[i], cindex, p_index, accountid);
			}
		}
		println!("total: {} guaranteed seats + {} newbies = {} total participants who would like to attend",
                 num_participants[0..=2].iter().sum::<u64>(),
                 num_participants[3],
                 num_participants[0..=3].iter().sum::<u64>()
        );
		Ok(())
	})
	.into()
}
pub fn list_meetups(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let cindex = matches.ceremony_index_arg().map_or_else(
			|| current_ceremony_index,
			|ci| into_effective_cindex(ci, current_ceremony_index),
		);

		let community_ceremony = (cid, cindex);

		println!("listing meetups for cid {cid} and ceremony nr {cindex}");

		let stats = api.get_community_ceremony_stats(community_ceremony).await.unwrap();

		let mut num_assignees = 0u64;

		for meetup in stats.meetups.iter() {
			println!(
				"MeetupRegistry[{:?}, {}] location is {:?}, {:?}",
				&community_ceremony, meetup.index, meetup.location.lat, meetup.location.lon
			);

			println!(
				"MeetupRegistry[{:?}, {}] meeting time is {:?}",
				&community_ceremony, meetup.index, meetup.time
			);

			if !meetup.registrations.is_empty() {
				let num = meetup.registrations.len();
				num_assignees += num as u64;
				println!(
					"MeetupRegistry[{:?}, {}] participants: {}",
					&community_ceremony, meetup.index, num
				);
				for (participant, _registration) in meetup.registrations.iter() {
					println!("   {participant}");
				}
			} else {
				println!("MeetupRegistry[{:?}, {}] EMPTY", &community_ceremony, meetup.index);
			}
		}
		println!("total number of assignees: {num_assignees}");
		Ok(())
	})
	.into()
}

pub fn print_ceremony_stats(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let cindex = matches.ceremony_index_arg().map_or_else(
			|| current_ceremony_index,
			|ci| into_effective_cindex(ci, current_ceremony_index),
		);

		let community_ceremony = (cid, cindex);

		let stats = api.get_community_ceremony_stats(community_ceremony).await.unwrap();

		// serialization prints the the account id better than `debug`
		println!("{}", serde_json::to_string_pretty(&stats).unwrap());
		Ok(())
	})
	.into()
}
pub fn list_attestees(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let cindex = matches.ceremony_index_arg().map_or_else(
			|| current_ceremony_index,
			|ci| into_effective_cindex(ci, current_ceremony_index),
		);

		let community_ceremony = (cid, cindex);

		let stats = api.get_community_ceremony_stats(community_ceremony).await.unwrap();

		// serialization prints the the account id better than `debug`
		println!("{}", serde_json::to_string_pretty(&stats).unwrap());
		Ok(())
	})
	.into()
}
pub fn list_reputables(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
        let api = get_chain_api(matches).await;

        let is_verbose = matches.verbose_flag();
        let at_block = matches.at_block_arg();

        let lifetime = get_reputation_lifetime(&api, at_block).await;
        let current_ceremony_index = get_ceremony_index(&api, at_block).await;


        let first_ceremony_index_of_interest = current_ceremony_index.saturating_sub(lifetime);
        let ceremony_indices: Vec<u32> = (first_ceremony_index_of_interest..current_ceremony_index).collect();

        let community_ids = get_cid_names(&api).await.unwrap().into_iter().map(|names| names.cid);

        let mut reputables_csv = Vec::new();

        println!("Listing the number of attested attendees for each community and ceremony for cycles [{:}:{:}]", ceremony_indices.first().unwrap(), ceremony_indices.last().unwrap());
        for community_id in community_ids {
            println!("Community ID: {community_id:?}");
            let mut reputables: HashMap<AccountId, usize> = HashMap::new();
            for ceremony_index in &ceremony_indices {
                let (attendees, noshows) = get_attendees_for_community_ceremony(&api, (community_id, *ceremony_index), at_block).await;
                println!("Cycle ID {ceremony_index:?}: Total attested attendees: {:} (noshows: {:})", attendees.len(), noshows.len());
                for attendee in attendees {
                    reputables_csv.push(format!("{community_id:?},{ceremony_index:?},{}", attendee.to_ss58check()));
                    *reputables.entry(attendee.clone()).or_insert(0) += 1;
                }
            }
            println!("Reputables in {community_id:?} (unique accounts with at least one attendance) {:}", reputables.keys().len());
        }
        if is_verbose {
            for reputable in reputables_csv {
                println!("{reputable}");
            }
        }
        Ok(())

    })
        .into()
}
pub fn upgrade_registration(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let arg_who = matches.account_arg().unwrap();
		let accountid = get_accountid_from_str(arg_who);
		let signer = match matches.signer_arg() {
			Some(sig) => get_pair_from_str(sig),
			None => get_pair_from_str(arg_who),
		};

		let api = get_chain_api(matches).await;
		let cindex = get_ceremony_index(&api, None).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;

		let current_phase = api.get_current_phase().await.unwrap();
		if !(current_phase == CeremonyPhaseType::Registering ||
			current_phase == CeremonyPhaseType::Attesting)
		{
			error!("wrong ceremony phase for registering participant");
			std::process::exit(exit_code::WRONG_PHASE);
		}
		let mut reputation_cindex = cindex;
		if current_phase == CeremonyPhaseType::Registering {
			reputation_cindex -= 1;
		}
		let rep = get_reputation(&api, &accountid, cid, reputation_cindex).await;
		info!("{} has reputation {:?}", accountid, rep);
		let proof = match rep {
			Reputation::VerifiedUnlinked =>
				prove_attendance(accountid, cid, reputation_cindex, arg_who),
			_ => {
				error!("No valid reputation in last ceremony.");
				std::process::exit(exit_code::INVALID_REPUTATION);
			},
		};

		let mut api = api;
		let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		api.set_signer(signer);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerCeremonies", "upgrade_registration", cid, proof)
				.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		// send and watch extrinsic until ready
		let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		info!("Upgrade registration sent for {}. status: '{:?}'", arg_who, report.status);
		Ok(())
	})
	.into()
}
pub fn register_participant(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let arg_who = matches.account_arg().unwrap();
		let accountid = get_accountid_from_str(arg_who);
		let signer = match matches.signer_arg() {
			Some(sig) => get_pair_from_str(sig),
			None => get_pair_from_str(arg_who),
		};

		let api = get_chain_api(matches).await;
		let cindex = get_ceremony_index(&api, None).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let rep = get_reputation(&api, &accountid, cid, cindex - 1).await;
		info!("{} has reputation {:?}", accountid, rep);
		let proof = match rep {
			Reputation::Unverified => None,
			Reputation::UnverifiedReputable => None, // this should never be the case during Registering!
			Reputation::VerifiedUnlinked =>
				Some(prove_attendance(accountid, cid, cindex - 1, arg_who)),
			Reputation::VerifiedLinked(_) =>
				Some(prove_attendance(accountid, cid, cindex - 1, arg_who)),
		};
		debug!("proof: {:x?}", proof.encode());
		let current_phase = api.get_current_phase().await.unwrap();
		if !(current_phase == CeremonyPhaseType::Registering ||
			current_phase == CeremonyPhaseType::Attesting)
		{
			error!("wrong ceremony phase for registering participant");
			std::process::exit(exit_code::WRONG_PHASE);
		}
		let mut api = api;
		let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		api.set_signer(signer);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerCeremonies", "register_participant", cid, proof)
				.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		// send and watch extrinsic until ready
		let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		info!("Registration sent for {}. status: '{:?}'", arg_who, report.status);
		Ok(())
	})
	.into()
}
pub fn unregister_participant(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let arg_who = matches.account_arg().unwrap();
		let signer = match matches.signer_arg() {
			Some(sig) => get_pair_from_str(sig),
			None => get_pair_from_str(arg_who),
		};

		let api = get_chain_api(matches).await;

		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;

		let cc = match matches.ceremony_index_arg() {
			Some(cindex_arg) => {
				let current_ceremony_index = get_ceremony_index(&api, None).await;
				let cindex = into_effective_cindex(cindex_arg, current_ceremony_index);
				Some((cid, cindex))
			},
			None => None,
		};

		let current_phase = api.get_current_phase().await.unwrap();
		if !(current_phase == CeremonyPhaseType::Registering ||
			current_phase == CeremonyPhaseType::Attesting)
		{
			error!("wrong ceremony phase for unregistering");
			std::process::exit(exit_code::WRONG_PHASE);
		}
		let mut api = api;
		let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		api.set_signer(signer);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerCeremonies", "unregister_participant", cid, cc)
				.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		// Send and watch extrinsic until ready
		let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		info!("Unregister Participant sent for {}. status: '{:?}'", arg_who, report.status);
		Ok(())
	})
	.into()
}
pub fn endorse(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let mut api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		endorse_newcomers(&mut api, cid, matches).await.unwrap();

		Ok(())
	})
	.into()
}
pub fn bootstrappers_with_remaining_newbie_tickets(
	_args: &str,
	matches: &ArgMatches<'_>,
) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let bs_with_tickets: Vec<BootstrapperWithTickets> =
			get_bootstrappers_with_remaining_newbie_tickets(&api, cid).await.unwrap();

		info!("burned_bootstrapper_newbie_tickets = {:?}", bs_with_tickets);

		// transform it to simple tuples, which is easier to parse in python
		let bt_vec = bs_with_tickets
			.into_iter()
			.map(|bt| (bt.bootstrapper.to_ss58check(), bt.remaining_newbie_tickets))
			.collect::<Vec<_>>();

		println!("{bt_vec:?}");
		Ok(())
	})
	.into()
}
pub fn get_proof_of_attendance(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let arg_who = matches.account_arg().unwrap();
		let accountid = get_accountid_from_str(arg_who);
		let api = get_chain_api(matches).await;

		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let cindex_arg = matches.ceremony_index_arg().unwrap_or(-1);
		let cindex = into_effective_cindex(cindex_arg, current_ceremony_index);

		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;

		debug!("Getting proof for ceremony index: {:?}", cindex);
		let proof = prove_attendance(accountid, cid, cindex, arg_who);
		info!("Proof: {:?}\n", &proof);
		println!("0x{}", hex::encode(proof.encode()));

		Ok(())
	})
	.into()
}
pub fn attest_attendees(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let attestees: Vec<_> = matches
			.attestees_arg()
			.unwrap()
			.into_iter()
			.map(get_accountid_from_str)
			.collect();

		let vote = attestees.len() as u32 + 1u32;

		debug!("attestees: {:?}", attestees);

		info!("send attest_attendees by {}", who.public());

		let mut api = get_chain_api(matches).await;
		let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone()));
		api.set_signer(signer);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerCeremonies",
			"attest_attendees",
			cid,
			vote,
			attestees
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();

		println!("Claims sent by {}. status: '{:?}'", who.public(), report.status);
		Ok(())
	})
	.into()
}
pub fn new_claim(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let arg_who = matches.account_arg().unwrap();
		let claimant = get_pair_from_str(arg_who);

		let n_participants = matches.value_of("vote").unwrap().parse::<u32>().unwrap();

		let claim = new_claim_for(&api, &claimant.into(), cid, n_participants).await;

		println!("{}", hex::encode(claim));

		Ok(())
	})
	.into()
}
pub fn claim_reward(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;

		let signer = match matches.signer_arg() {
			Some(sig) => get_pair_from_str(sig),
			None => panic!("please specify --signer."),
		};
		let mut api = api;
		let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		api.set_signer(signer.clone());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		let meetup_index_arg = matches.meetup_index_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		if matches.all_flag() {
			let mut cindex = get_ceremony_index(&api, None).await;
			if api.get_current_phase().await.unwrap() == CeremonyPhaseType::Registering {
				cindex -= 1;
			}
			let meetup_count = api
				.get_storage_map("EncointerCeremonies", "MeetupCount", (cid, cindex), None)
				.await
				.unwrap()
				.unwrap_or(0u64);
			let calls: Vec<_> = (1u64..=meetup_count)
				.map(|idx| {
					compose_call!(
						api.metadata(),
						ENCOINTER_CEREMONIES,
						"claim_rewards",
						cid,
						Option::<MeetupIndexType>::Some(idx)
					)
					.unwrap()
				})
				.collect();
			let batch_call = compose_call!(api.metadata(), "Utility", "batch", calls).unwrap();
			send_and_wait_for_in_block(&api, xt(&api, batch_call).await, tx_payment_cid_arg).await;
			println!("Claiming reward for all meetup indexes. xt-status: 'ready'");
		} else {
			let meetup_index = meetup_index_arg;
			let xt: EncointerXt<_> =
				compose_extrinsic!(api, ENCOINTER_CEREMONIES, "claim_rewards", cid, meetup_index)
					.unwrap();
			ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
			let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
			match meetup_index_arg {
				Some(idx) => {
					println!(
						"Claiming reward for meetup_index {idx}. xt-status: '{:?}'",
						report.status
					);
				},
				None => {
					println!(
						"Claiming reward for {}. xt-status: 'ready'",
						signer.public_account_id()
					);
				},
			}
		}
		Ok(())
	})
	.into()
}
pub fn reputation(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let account = matches.account_arg().unwrap();
		let account_id = get_accountid_from_str(account);
		if let Some(reputation) = get_reputation_history(&api, &account_id).await {
			for rep in reputation.iter() {
				println!("{}, {}, {:?}", rep.0, rep.1.community_identifier, rep.1.reputation);
			}
		} else {
			error!("could not fetch reputation over rpc");
			std::process::exit(exit_code::RPC_ERROR);
		}
		Ok(())
	})
	.into()
}

pub fn set_meetup_time_offset(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let mut api = get_chain_api(matches).await;
		let signer = ParentchainExtrinsicSigner::new(AccountKeyring::Alice.pair());
		api.set_signer(signer);
		let time_offset = matches.time_offset_arg().unwrap_or(0);
		let call = compose_call!(
			api.metadata(),
			"EncointerCeremonies",
			"set_meetup_time_offset",
			time_offset
		)
		.unwrap();

		// return calls as `OpaqueCall`s to get the same return type in both branches
		let privileged_call = if contains_sudo_pallet(api.metadata()) {
			let sudo_call = sudo_call(api.metadata(), call);
			info!("Printing raw sudo call for js/apps:");
			print_raw_call("sudo(...)", &sudo_call);
			OpaqueCall::from_tuple(&sudo_call)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
			let propose_call = collective_propose_call(api.metadata(), threshold, call);
			print_raw_call("collective_propose(...)", &propose_call);
			OpaqueCall::from_tuple(&propose_call)
		};

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
		let xt = xt(&api, privileged_call).await;
		send_and_wait_for_in_block(&api, xt, tx_payment_cid_arg).await;
		Ok(())
	})
	.into()
}

pub fn purge_community_ceremony(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let sudoer = AccountKeyring::Alice.pair();
		let signer = ParentchainExtrinsicSigner::new(sudoer);
		let mut api = get_chain_api(matches).await;
		api.set_signer(signer);

		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let from_cindex_arg = matches.from_cindex_arg().unwrap_or(0);
		let to_cindex_arg = matches.to_cindex_arg().unwrap_or(0);

		let from_cindex = into_effective_cindex(from_cindex_arg, current_ceremony_index);
		let to_cindex = into_effective_cindex(to_cindex_arg, current_ceremony_index);

		if from_cindex > to_cindex {
			panic!("'from' <= 'to' ceremony index violated");
		}
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		println!("purging ceremony index range [{from_cindex}  {to_cindex}] for community {cid}");

		let calls: Vec<_> = (from_cindex..=to_cindex)
			.map(|idx| {
				compose_call!(
					api.metadata(),
					"EncointerCeremonies",
					"purge_community_ceremony",
					(cid, idx)
				)
				.unwrap()
			})
			.collect();
		let batch_call = compose_call!(api.metadata(), "Utility", "batch", calls).unwrap();
		let unsigned_sudo_call =
			compose_call!(api.metadata(), "Sudo", "sudo", batch_call.clone()).unwrap();
		info!(
			"raw sudo batch call to sign with js/apps {}: 0x{}",
			cid,
			hex::encode(unsigned_sudo_call.encode())
		);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
		let xt: EncointerXt<_> = compose_extrinsic!(api, "Sudo", "sudo", batch_call).unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let tx_report = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap();
		info!("[+] Transaction got included. Block Hash: {:?}\n", tx_report.block_hash.unwrap());
		Ok(())
	})
	.into()
}

fn prove_attendance(
	prover: AccountId,
	cid: CommunityIdentifier,
	cindex: CeremonyIndexType,
	attendee_str: &str,
) -> ProofOfAttendance<Signature, AccountId> {
	let msg = (prover.clone(), cindex);
	let attendee = get_pair_from_str(attendee_str);
	let attendeeid = get_accountid_from_str(attendee_str);
	debug!("generating proof of attendance for {} and cindex: {}", prover, cindex);
	debug!("signature payload is {:x?}", msg.encode());
	ProofOfAttendance {
		prover_public: prover,
		community_identifier: cid,
		ceremony_index: cindex,
		attendee_public: attendeeid,
		attendee_signature: Signature::from(sr25519_core::Signature::from(
			attendee.sign(&msg.encode()),
		)),
	}
}

async fn get_reputation(
	api: &Api,
	prover: &AccountId,
	cid: CommunityIdentifier,
	cindex: CeremonyIndexType,
) -> Reputation {
	api.get_storage_double_map(
		"EncointerCeremonies",
		"ParticipantReputation",
		(cid, cindex),
		prover.clone(),
		None,
	)
	.await
	.unwrap()
	.unwrap_or(Reputation::Unverified)
}

async fn get_reputation_history(
	api: &Api,
	account_id: &AccountId,
) -> Option<Vec<(CeremonyIndexType, CommunityReputation)>> {
	api.client()
		.request("encointer_getReputations", rpc_params![account_id])
		.await
		.expect("Could not query reputation history...")
}

async fn get_attendees_for_community_ceremony(
	api: &Api,
	community_ceremony: CommunityCeremony,
	at_block: Option<Hash>,
) -> (Vec<AccountId>, Vec<AccountId>) {
	let key_prefix = api
		.get_storage_double_map_key_prefix(
			"EncointerCeremonies",
			"ParticipantReputation",
			community_ceremony,
		)
		.await
		.unwrap();
	let max_keys = 1000;
	let storage_keys = api
		.get_storage_keys_paged(Some(key_prefix), max_keys, None, at_block)
		.await
		.unwrap();

	if storage_keys.len() == max_keys as usize {
		error!("results can be wrong because max keys reached for query")
	}
	let mut attendees = Vec::new();
	let mut noshows = Vec::new();
	for storage_key in storage_keys.iter() {
		match api.get_storage_by_key(storage_key.clone(), at_block).await.unwrap().unwrap() {
			Reputation::VerifiedUnlinked | Reputation::VerifiedLinked(_) => {
				let key_postfix = storage_key.as_ref();
				attendees.push(
					AccountId::decode(&mut key_postfix[key_postfix.len() - 32..].as_ref()).unwrap(),
				);
			},
			Reputation::UnverifiedReputable | Reputation::Unverified => {
				let key_postfix = storage_key.as_ref();
				noshows.push(
					AccountId::decode(&mut key_postfix[key_postfix.len() - 32..].as_ref()).unwrap(),
				);
			},
		}
	}
	(attendees, noshows)
}

async fn get_reputation_lifetime(api: &Api, at_block: Option<Hash>) -> ReputationLifetimeType {
	api.get_storage("EncointerCeremonies", "ReputationLifetime", at_block)
		.await
		.unwrap()
		.unwrap_or(5)
}

async fn new_claim_for(
	api: &Api,
	claimant: &sr25519::Pair,
	cid: CommunityIdentifier,
	n_participants: u32,
) -> Vec<u8> {
	let cindex = get_ceremony_index(api, None).await;
	let mindex = api
		.get_meetup_index(&(cid, cindex), &claimant.public().into())
		.await
		.unwrap()
		.expect("participant must be assigned to meetup to generate a claim");

	// implicitly assume that participant meet at the right place at the right time
	let mloc = api.get_meetup_location(&(cid, cindex), mindex).await.unwrap().unwrap();
	let mtime = api.get_meetup_time(mloc, ONE_DAY).await.unwrap();

	info!(
		"creating claim for {} at loc {} (lat: {} lon: {}) at time {}, cindex {}",
		claimant.public().to_ss58check(),
		mindex,
		mloc.lat,
		mloc.lon,
		mtime,
		cindex
	);
	let claim: ClaimOfAttendance<MultiSignature, AccountId, Moment> =
		ClaimOfAttendance::new_unsigned(
			claimant.public().into(),
			cindex,
			cid,
			mindex,
			mloc,
			mtime,
			n_participants,
		)
		.sign(claimant);
	claim.encode()
}

async fn endorse_newcomers(
	api: &mut Api,
	cid: CommunityIdentifier,
	matches: &ArgMatches<'_>,
) -> Result<(), ApiClientError> {
	let bootstrapper = matches.bootstrapper_arg().map(get_pair_from_str).unwrap();
	let endorsees = matches.endorsees_arg().expect("Please supply at least one endorsee");

	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(bootstrapper)));

	let mut nonce = api.get_nonce().await?;

	let tx_payment_cid_arg = matches.tx_payment_cid_arg();
	set_api_extrisic_params_builder(api, tx_payment_cid_arg).await;

	for e in endorsees.into_iter() {
		let endorsee = get_accountid_from_str(e);

		let call =
			compose_call!(api.metadata(), "EncointerCeremonies", "endorse_newcomer", cid, endorsee)
				.unwrap();

		let encoded_xt: Bytes = api.compose_extrinsic_offline(call, nonce).encode().into();
		ensure_payment(api, &encoded_xt, tx_payment_cid_arg).await;
		let _tx_report = api
			.submit_and_watch_opaque_extrinsic_until(&encoded_xt, XtStatus::Ready)
			.await
			.unwrap();

		nonce += 1;
	}

	Ok(())
}

/// Helper type, which is only needed to print the information nicely.
#[derive(Debug)]
struct BootstrapperWithTickets {
	bootstrapper: AccountId,
	remaining_newbie_tickets: u8,
}

async fn get_bootstrappers_with_remaining_newbie_tickets(
	api: &Api,
	cid: CommunityIdentifier,
) -> Result<Vec<BootstrapperWithTickets>, ApiClientError> {
	let total_newbie_tickets: u8 = api
		.get_storage("EncointerCeremonies", "EndorsementTicketsPerBootstrapper", None)
		.await
		.unwrap()
		.unwrap();

	// prepare closure to make below call more readable.
	let ticket_query = |bs| async move {
		let remaining_tickets = total_newbie_tickets -
			api.get_storage_double_map(
				"EncointerCeremonies",
				"BurnedBootstrapperNewbieTickets",
				cid,
				bs,
				None,
			)
			.await?
			.unwrap_or(0u8);

		Ok::<_, ApiClientError>(remaining_tickets)
	};

	let bootstrappers: Vec<AccountId> = api
		.get_storage_map("EncointerCommunities", "Bootstrappers", cid, None)
		.await?
		.expect("No bootstrappers found, does the community exist?");

	let mut bs_with_tickets: Vec<BootstrapperWithTickets> = Vec::with_capacity(bootstrappers.len());

	for bs in bootstrappers.into_iter() {
		bs_with_tickets.push(BootstrapperWithTickets {
			bootstrapper: bs.clone(),
			remaining_newbie_tickets: ticket_query(bs).await?,
		});
	}

	Ok(bs_with_tickets)
}
