//  Copyright (c) 2019 Alain Brenzikofer
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

//! an RPC client to encointer node using websockets
//!
//! examples:
//! encointer-client-notee get-phase
//! encointer-client-notee transfer //Alice 5G9RtsTbiYJYQYMHbWfyPoeuuxNaCbC16tZ2JGrZ4gRKwz14 1000
//!

mod cli_args;
mod commands;
mod community_spec;
mod utils;

use crate::{
	community_spec::{
		CommunitySpec,
	},
	utils::{
		ensure_payment,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
};
use clap::{value_t, AppSettings, Arg, ArgMatches};
use clap_nested::{Command, Commander};
use cli_args::{EncointerArgs, EncointerArgsExtractor};
use encointer_api_client_extension::{
	Api, CeremoniesApi, CommunityCurrencyTip,
	CommunityCurrencyTipExtrinsicParamsBuilder, EncointerXt,
	ParentchainExtrinsicSigner,
};
use encointer_node_notee_runtime::{
	AccountId, BalanceEntry, BalanceType, BlockNumber, Hash, Moment, RuntimeEvent,
	Signature, ONE_DAY,
};
use encointer_primitives::{
	balances::{to_U64F64, Demurrage},
	bazaar::{Business, BusinessIdentifier, OfferingData},
	ceremonies::{
		ClaimOfAttendance, CommunityCeremony, CommunityReputation,
		ParticipantIndexType, ProofOfAttendance, Reputation, ReputationLifetimeType,
	},
	communities::{CidName, CommunityIdentifier},
	faucet::{FromStr as FaucetNameFromStr},
	fixed::transcendental::exp,
	scheduler::{CeremonyIndexType},
};

use log::*;
use pallet_transaction_payment::FeeDetails;
use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::{sr25519};
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_keyring::AccountKeyring;
use sp_keystore::Keystore;
use sp_rpc::number::NumberOrHex;
use sp_runtime::MultiSignature;
use std::{str::FromStr};
use substrate_api_client::{
	ac_compose_macros::{compose_call, compose_extrinsic, rpc_params},
	ac_primitives::{Bytes, SignExtrinsic},
	api::error::Error as ApiClientError,
	extrinsic::BalancesExtrinsics,
	rpc::{JsonrpseeClient, Request}, GetBalance, GetChainInfo, GetStorage, GetTransactionPayment,
	SubmitAndWatch, SubscribeEvents, XtStatus,
};


const PREFUNDING_NR_OF_TRANSFER_EXTRINSICS: u128 = 1000;
const VERSION: &str = env!("CARGO_PKG_VERSION");

mod exit_code {
	pub const WRONG_PHASE: i32 = 50;
	pub const FEE_PAYMENT_FAILED: i32 = 51;
	pub const INVALID_REPUTATION: i32 = 52;
	pub const RPC_ERROR: i32 = 60;
	pub const NO_CID_SPECIFIED: i32 = 70;
}

#[tokio::main]
async fn main() {
	env_logger::init();

	Commander::new()
		.options(|app| {
			app.arg(
				Arg::with_name("node-url")
					.short("u")
					.long("node-url")
					.global(true)
					.takes_value(true)
					.value_name("STRING")
					.default_value("ws://127.0.0.1")
					.help("node url"),
			)
			.arg(
				Arg::with_name("node-port")
					.short("p")
					.long("node-port")
					.global(true)
					.takes_value(true)
					.value_name("STRING")
					.default_value("9944")
					.help("node port"),
			)
			.optional_cid_arg()
			.tx_payment_cid_arg()
			.name("encointer-client-notee")
			.version(VERSION)
			.author("Encointer Association <info@encointer.org>")
			.about("interact with encointer-node-notee")
			.after_help("")
			.setting(AppSettings::ColoredHelp)
		})
		.args(|_args, _matches| "")
		.add_cmd(
		    Command::new("new-account")
		        .description("Imports account into the key store. Either creates a new account or with the supplied seed.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .seed_arg()
		        })
		        .runner(commands::keystore::new_account),
		)
		.add_cmd(
		    Command::new("list-accounts")
		        .description("lists all accounts in keystore")
		        .runner(commands::keystore::list_accounts),
		)
		.add_cmd(
		    Command::new("print-metadata")
		        .description("query node metadata and print it as json to stdout")
		        .runner(commands::frame::print_metadata),
		)
		.add_cmd(
		    Command::new("faucet")
		        .description("send some bootstrapping funds to supplied account(s)")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .fundees_arg()
		        })
		        .runner(commands::frame::faucet),
		)
		.add_cmd(
		    Command::new("balance")
		        .description("query on-chain balance for AccountId. If --cid is supplied, returns balance in that community. Otherwise balance of native ERT token")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .all_flag()
		            .at_block_arg()
		        })
		        .runner(commands::encointer_core::balance),
		)
		.add_cmd(
		    Command::new("issuance")
		        .description("query total issuance for community. must supply --cid")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .at_block_arg()
		        })
		        .runner(commands::encointer_core::issuance),
		)
		.add_cmd(
		    Command::new("transfer")
		        .description("transfer funds from one account to another. If --cid is supplied, send that community (amount is fixpoint). Otherwise send native ERT tokens (amount is integer)")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .dryrun_flag()
		            .arg(
		                Arg::with_name("from")
		                    .takes_value(true)
		                    .required(true)
		                    .value_name("SS58")
		                    .help("sender's AccountId in ss58check format"),
		            )
		            .arg(
		                Arg::with_name("to")
		                    .takes_value(true)
		                    .required(true)
		                    .value_name("SS58")
		                    .help("recipient's AccountId in ss58check format"),
		            )
		            .arg(
		                Arg::with_name("amount")
		                    .takes_value(true)
		                    .required(true)
		                    .value_name("U128")
		                    .help("amount to be transferred"),
		            )
		        })
		        .runner(commands::encointer_core::transfer),
		)
		.add_cmd(
		    Command::new("transfer_all")
		        .description("transfer all available funds from one account to another for a community specified with --cid.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .arg(
		                Arg::with_name("from")
		                    .takes_value(true)
		                    .required(true)
		                    .value_name("SS58")
		                    .help("sender's AccountId in ss58check format"),
		            )
		            .arg(
		                Arg::with_name("to")
		                    .takes_value(true)
		                    .required(true)
		                    .value_name("SS58")
		                    .help("recipient's AccountId in ss58check format"),
		            )
		        })
		        .runner(commands::encointer_core::transfer_all),
		)
		.add_cmd(
		    Command::new("listen")
		        .description("listen to on-chain events")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .arg(
		                Arg::with_name("events")
		                    .short("e")
		                    .long("await-events")
		                    .takes_value(true)
		                    .help("exit after given number of encointer events"),
		            )
		            .arg(
		                Arg::with_name("blocks")
		                    .short("b")
		                    .long("await-blocks")
		                    .takes_value(true)
		                    .help("exit after given number of blocks"),
		            )
		        })
		        .runner(commands::encointer_core::listen_to_events),
		)
		.add_cmd(
		    Command::new("new-community")
		        .description("Register new community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .arg(
		                Arg::with_name("specfile")
		                    .takes_value(true)
		                    .required(true)
		                    .help("enhanced geojson file that specifies a community"),
		            )
		            .signer_arg("account with necessary privileges")
		        })
		        .runner(commands::encointer_communities::new_community),
		)
		.add_cmd(
		    Command::new("add-locations")
		        .description("Register new locations for a community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges")
		                .dryrun_flag()
		                .arg(
		                    Arg::with_name("specfile")
		                        .takes_value(true)
		                        .required(true)
		                        .help("geojson file that specifies locations to add as points"),
		                )
		        })
		        .runner(commands::encointer_communities::add_locations),
		)
		.add_cmd(
		    Command::new("list-communities")
		        .description("list all registered communities")
		        .runner(commands::encointer_communities::list_communities),
		)
		.add_cmd(
		    Command::new("list-locations")
		        .description("list all meetup locations for a community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		        })
		        .runner(commands::encointer_communities::list_locations),
		)
		.add_cmd(
		    Command::new("get-phase")
		        .description("read current ceremony phase from chain")
		        .runner(commands::encointer_core::get_phase),
		)
		.add_cmd(
		    Command::new("next-phase")
		        .description("Advance ceremony state machine to next phase by ROOT call")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		        })
		       .runner(commands::encointer_core::next_phase),
		)
		.add_cmd(
		    Command::new("list-participants")
		        .description("list all registered participants supplied community identifier and ceremony index")
		        .options(|app| {
		        app.setting(AppSettings::ColoredHelp)
		            .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::list_participants),
		)
		.add_cmd(
		    Command::new("list-meetups")
		        .description("list all assigned meetups for supplied community identifier and ceremony index")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::list_meetups),
		)
		.add_cmd(
		    Command::new("print-ceremony-stats")
		        .description("pretty prints all information for a community ceremony")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::print_ceremony_stats),
		)
		.add_cmd(
		    Command::new("list-attestees")
		        .description("list all attestees for participants for supplied community identifier and ceremony index")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::list_attestees),
		)
		.add_cmd(
		    Command::new("list-reputables")
		        .description("list all reputables for all cycles within the current reputation-lifetime for all communities")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		                .verbose_flag()
		        })
		        .runner(commands::encointer_ceremonies::list_reputables),
		        )
		.add_cmd(
		    Command::new("register-participant")
		        .description("Register encointer ceremony participant for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .signer_arg("Account which signs the tx.")
		        })
		        .runner(commands::encointer_ceremonies::register_participant),
		)
		.add_cmd(
		    Command::new("upgrade-registration")
		        .description("Upgrade registration to repuable for encointer ceremony participant for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .signer_arg("Account which signs the tx.")
		        })
		        .runner(commands::encointer_ceremonies::upgrade_registration),
		)
		.add_cmd(
		    Command::new("unregister-participant")
		        .description("Unregister encointer ceremony participant for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .signer_arg("Account which signs the tx.")
		            .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::unregister_participant),
		)
		.add_cmd(
		    Command::new("endorse-newcomers")
		        .description("Endorse newbies with a bootstrapper account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .bootstrapper_arg()
		                .endorsees_arg()
		        })
		        .runner(commands::encointer_ceremonies::endorse),
		)
		.add_cmd(
		    Command::new("get-bootstrappers-with-remaining-newbie-tickets")
		        .description("Get the bootstrappers along with the remaining newbie tickets")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		        })
		       .runner(commands::encointer_ceremonies::bootstrappers_with_remaining_newbie_tickets),
		)
		.add_cmd(
		    Command::new("get-proof-of-attendance")
		        .description("creates a proof of ProofOfAttendances for an <account> for the given ceremony index")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .setting(AppSettings::AllowLeadingHyphen)
		                .account_arg()
		                .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::get_proof_of_attendance),
		)
		.add_cmd(
		    Command::new("attest-attendees")
		        .description("Register encointer ceremony claim of attendances for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .optional_cid_arg()
		                .attestees_arg()
		        })
		        .runner(commands::encointer_ceremonies::attest_attendees),
		)
		.add_cmd(
		    Command::new("new-claim")
		        .description("create a fresh claim of attendance for account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .arg(
		                Arg::with_name("vote")
		                    .takes_value(true)
		                    .required(true)
		                    .value_name("VOTE")
		                    .help("participant's vote on the number of people present at meetup time"),
		            )
		        })
		        .runner(commands::encointer_ceremonies::new_claim),
		)
		.add_cmd(
		    Command::new("claim-reward")
		        .description("Claim the rewards for all meetup participants of the last ceremony.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("Account which signs the tx.")
		                .meetup_index_arg()
		                .all_flag()
		        })
		        .runner(commands::encointer_ceremonies::claim_reward),
		)
		.add_cmd(
		    Command::new("reputation")
		        .description("List reputation history for an account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()})
		        .runner(commands::encointer_ceremonies::reputation),
		)
		.add_cmd(
			Command::new("purge-community-ceremony")
				.description("purge all history within the provided ceremony index range for the specified community")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.from_cindex_arg()
						.to_cindex_arg()

				})
				.runner(commands::encointer_ceremonies::purge_community_ceremony),
		)
		.add_cmd(
			Command::new("set-meetup-time-offset")
				.description("signed value to offset the ceremony meetup time relative to solar noon")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.setting(AppSettings::AllowLeadingHyphen)
						.time_offset_arg()
				})
				.runner(commands::encointer_ceremonies::set_meetup_time_offset),
		)
		.add_cmd(
		    Command::new("create-business")
		        .description("Register a community business on behalf of the account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(commands::encointer_bazaar::create_business),
		)
		.add_cmd(
		    Command::new("update-business")
		        .description("Update an already existing community business on behalf of the account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(commands::encointer_bazaar::update_business),
		)
		.add_cmd(
		    Command::new("create-offering")
		        .description("Create an offering for the business belonging to account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(commands::encointer_bazaar::create_offering),
		)
		.add_cmd(
		    Command::new("list-businesses")
		        .description("List businesses for a community")
		        .runner(commands::encointer_bazaar::list_businesses),
		)
		.add_cmd(
		    Command::new("list-offerings")
		        .description("List offerings for a community")
		        .runner(commands::encointer_bazaar::list_offerings),
		)
		.add_cmd(
		    Command::new("list-business-offerings")
		        .description("List offerings for a business")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		        })
		        .runner(commands::encointer_bazaar::list_business_offerings),
		)
		.add_cmd(
		    Command::new("create-faucet")
		        .description("Create faucet")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_name_arg()
		                .faucet_balance_arg()
		                .faucet_drip_amount_arg()
		                .whitelist_arg()
		        })
		        .runner(commands::encointer_faucet::create_faucet),
		)
		.add_cmd(
		    Command::new("drip-faucet")
		        .description("Drip faucet. args: 1. faucet account, 2. cindex of the reputation. use --cid to specify the community.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_account_arg()
		                .cindex_arg()
		        })
		        .runner(commands::encointer_faucet::drip_faucet),
		)
		.add_cmd(
		    Command::new("dissolve-faucet")
		        .description("can only be called by root. args: 1. faucet address, 2. beneficiary of the remaining funds.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		                .faucet_account_arg()
		                .faucet_beneficiary_arg()
		        })
		       .runner(commands::encointer_faucet::dissolve_faucet),
		)
		.add_cmd(
		    Command::new("close-faucet")
		        .description("lazy garbage collection. can only be called by faucet creator and only once the faucet is empty")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_account_arg()
		        })
		        .runner(commands::encointer_faucet::close_faucet),
		)
		.add_cmd(
		    Command::new("set-faucet-reserve-amount")
		        .description("Set faucet pallet reserve amount")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		                .faucet_reserve_amount_arg()
		        })
		       .runner(commands::encointer_faucet::set_faucet_reserve_amount),
		)
		.add_cmd(
		    Command::new("list-faucets")
		        .description("list all faucets. use -v to get faucet details.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		                .verbose_flag()
		        })
		       .runner(commands::encointer_faucet::list_faucets)
		)
		.add_cmd(
			Command::new("submit-set-inactivity-timeout-proposal")
				.description("Submit set inactivity timeout proposal")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.inactivity_timeout_arg()
				})
				.runner(commands::encointer_democracy::submit_set_inactivity_timeout_proposal),
		)
		.add_cmd(
			Command::new("list-proposals")
				.description("list all proposals.")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.at_block_arg()
				})
			   .runner(commands::encointer_democracy::list_proposals),
				)
		.add_cmd(
			Command::new("vote")
				.description("Submit vote for porposal. Vote is either ay or nay. Reputation vec to be specified as cid1_cindex1,cid2_cindex2,...")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.proposal_id_arg()
						.vote_arg()
						.reputation_vec_arg()
				})
				.runner(commands::encointer_democracy::vote),
		)
		.add_cmd(
			Command::new("update-proposal-state")
				.description("Update proposal state")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp).account_arg().proposal_id_arg()
				})
				.runner(commands::encointer_democracy::update_proposal_state),
		)
		// To handle when no subcommands match
		.no_cmd(|_args, _matches| {
			println!("No subcommand matched");
			Ok(())
		})
		.run();
}
//////////////////////
async fn get_chain_api(matches: &ArgMatches<'_>) -> Api {
	let url = format!(
		"{}:{}",
		matches.value_of("node-url").unwrap(),
		matches.value_of("node-port").unwrap()
	);
	debug!("connecting to {}", url);
	let client = JsonrpseeClient::new(&url).await.expect("node URL is incorrect");
	Api::new(client).await.unwrap()
}

async fn reasonable_native_balance(api: &Api) -> u128 {
	let alice: AccountId = AccountKeyring::Alice.into();
	let xt = api.balance_transfer_allow_death(alice.into(), 9999).await.unwrap();
	let fee = api
		.get_fee_details(&xt.encode().into(), None)
		.await
		.unwrap()
		.unwrap()
		.inclusion_fee
		.unwrap()
		.base_fee;
	let ed = api.get_existential_deposit().await.unwrap();
	ed + fee * PREFUNDING_NR_OF_TRANSFER_EXTRINSICS
}

async fn listen(matches: &ArgMatches<'_>) {
	let api = get_chain_api(matches).await;
	debug!("Subscribing to events");
	let mut subscription = api.subscribe_events().await.unwrap();
	let mut count = 0u32;
	let mut blocks = 0u32;
	loop {
		if matches.is_present("events")
			&& count >= value_t!(matches.value_of("events"), u32).unwrap()
		{
			return;
		};
		if matches.is_present("blocks")
			&& blocks > value_t!(matches.value_of("blocks"), u32).unwrap()
		{
			return;
		};
		let event_results = subscription.next_events::<RuntimeEvent, Hash>().await.unwrap();
		blocks += 1;
		match event_results {
			Ok(evts) => {
				for evr in evts {
					debug!("decoded: phase {:?} event {:?}", evr.phase, evr.event);
					match &evr.event {
						RuntimeEvent::EncointerCeremonies(ee) => {
							count += 1;
							info!(">>>>>>>>>> ceremony event: {:?}", ee);
							match &ee {
								pallet_encointer_ceremonies::Event::ParticipantRegistered(
									cid,
									participant_type,
									accountid,
								) => {
									println!(
										"Participant registered as {participant_type:?}, for cid: {cid:?}, account: {accountid}, "
									);
								},
								_ => println!("Unsupported EncointerCommunities event"),
							}
						},
						RuntimeEvent::EncointerScheduler(ee) => {
							count += 1;
							info!(">>>>>>>>>> scheduler event: {:?}", ee);
							match &ee {
								pallet_encointer_scheduler::Event::PhaseChangedTo(phase) => {
									println!("Phase changed to: {phase:?}");
								},
								pallet_encointer_scheduler::Event::CeremonySchedulePushedByOneDay => {
									println!("Ceremony schedule was pushed by one day");
								},
							}
						},
						RuntimeEvent::EncointerCommunities(ee) => {
							count += 1;
							info!(">>>>>>>>>> community event: {:?}", ee);
							match &ee {
								pallet_encointer_communities::Event::CommunityRegistered(cid) => {
									println!("Community registered: cid: {cid:?}");
								},
								pallet_encointer_communities::Event::MetadataUpdated(cid) => {
									println!("Community metadata updated cid: {cid:?}");
								},
								pallet_encointer_communities::Event::NominalIncomeUpdated(
									cid,
									income,
								) => {
									println!(
										"Community metadata updated cid: {cid:?}, value: {income:?}"
									);
								},
								pallet_encointer_communities::Event::DemurrageUpdated(
									cid,
									demurrage,
								) => {
									println!(
										"Community metadata updated cid: {cid:?}, value: {demurrage:?}"
									);
								},
								_ => println!("Unsupported EncointerCommunities event"),
							}
						},
						RuntimeEvent::EncointerBalances(ee) => {
							count += 1;
							println!(">>>>>>>>>> encointer balances event: {ee:?}");
						},
						RuntimeEvent::EncointerBazaar(ee) => {
							count += 1;
							println!(">>>>>>>>>> encointer bazaar event: {ee:?}");
						},
						RuntimeEvent::System(ee) => match ee {
							frame_system::Event::ExtrinsicFailed {
								dispatch_error: _,
								dispatch_info: _,
							} => {
								error!("ExtrinsicFailed: {ee:?}");
							},
							frame_system::Event::ExtrinsicSuccess { dispatch_info } => {
								println!("ExtrinsicSuccess: {dispatch_info:?}");
							},
							_ => debug!("ignoring unsupported system Event"),
						},
						_ => debug!("ignoring unsupported module event: {:?}", evr.event),
					}
				}
			},
			Err(_) => error!("couldn't decode event record list"),
		}
	}
}

async fn verify_cid(api: &Api, cid: &str, maybe_at: Option<Hash>) -> CommunityIdentifier {
	let cids = get_community_identifiers(api, maybe_at).await.expect("no community registered");
	let cid = CommunityIdentifier::from_str(cid).unwrap();
	if !cids.contains(&cid) {
		panic!("cid {cid} does not exist on chain");
	}
	cid
}

async fn get_block_number(api: &Api, maybe_at: Option<Hash>) -> BlockNumber {
	let hdr = api.get_header(maybe_at).await.unwrap().unwrap();
	debug!("decoded: {:?}", hdr);
	//let hdr: Header= Decode::decode(&mut .as_bytes()).unwrap();
	hdr.number
}

pub async fn get_community_balance(
	api: &Api,
	cid_str: &str,
	account_id: &AccountId,
	maybe_at: Option<Hash>,
) -> BalanceType {
	let cid = verify_cid(api, cid_str, maybe_at).await;
	let bn = get_block_number(api, maybe_at).await;
	let dr = get_demurrage_per_block(api, cid).await;

	if let Some(entry) = api
		.get_storage_double_map("EncointerBalances", "Balance", cid, account_id, maybe_at)
		.await
		.unwrap()
	{
		apply_demurrage(entry, bn, dr)
	} else {
		BalanceType::from_num(0)
	}
}

pub async fn get_community_issuance(
	api: &Api,
	cid_str: &str,
	maybe_at: Option<Hash>,
) -> BalanceType {
	let cid = verify_cid(api, cid_str, maybe_at).await;
	let bn = get_block_number(api, maybe_at).await;
	let dr = get_demurrage_per_block(api, cid).await;

	if let Some(entry) = api
		.get_storage_map("EncointerBalances", "TotalIssuance", cid, maybe_at)
		.await
		.unwrap()
	{
		apply_demurrage(entry, bn, dr)
	} else {
		BalanceType::from_num(0)
	}
}

async fn get_demurrage_per_block(api: &Api, cid: CommunityIdentifier) -> Demurrage {
	let d: Option<Demurrage> = api
		.get_storage_map("EncointerBalances", "DemurragePerBlock", cid, None)
		.await
		.unwrap();

	match d {
		Some(d) => {
			debug!("Fetched community specific demurrage per block {:?}", &d);
			d
		},
		None => {
			let d = api.get_constant("EncointerBalances", "DefaultDemurrage").await.unwrap();
			debug!("Fetched default demurrage per block {:?}", d);
			d
		},
	}
}

async fn get_ceremony_index(api: &Api, at_block: Option<Hash>) -> CeremonyIndexType {
	api.get_storage("EncointerScheduler", "CurrentCeremonyIndex", at_block)
		.await
		.unwrap()
		.unwrap()
}

async fn get_attestee_count(api: &Api, key: CommunityCeremony) -> ParticipantIndexType {
	api.get_storage_map("EncointerCeremonies", "AttestationCount", key, None)
		.await
		.unwrap()
		.unwrap_or(0)
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

async fn get_participant_attestation_index(
	api: &Api,
	key: CommunityCeremony,
	accountid: &AccountId,
) -> Option<ParticipantIndexType> {
	api.get_storage_double_map("EncointerCeremonies", "AttestationIndex", key, accountid, None)
		.await
		.unwrap()
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

async fn get_community_identifiers(
	api: &Api,
	maybe_at: Option<Hash>,
) -> Option<Vec<CommunityIdentifier>> {
	api.get_storage("EncointerCommunities", "CommunityIdentifiers", maybe_at)
		.await
		.unwrap()
}

/// This rpc needs to have offchain indexing enabled in the node.
async fn get_cid_names(api: &Api) -> Option<Vec<CidName>> {
	api.client().request("encointer_getAllCommunities", rpc_params![]).await.expect(
		"No communities returned. Are you running the node with `--enable-offchain-indexing true`?",
	)
}

async fn get_businesses(api: &Api, cid: CommunityIdentifier) -> Option<Vec<Business<AccountId>>> {
	api.client()
		.request("encointer_bazaarGetBusinesses", rpc_params![cid])
		.await
		.expect("Could not find any businesses...")
}

async fn get_offerings(api: &Api, cid: CommunityIdentifier) -> Option<Vec<OfferingData>> {
	api.client()
		.request("encointer_bazaarGetOfferings", rpc_params![cid])
		.await
		.expect("Could not find any business offerings...")
}

async fn get_offerings_for_business(
	api: &Api,
	cid: CommunityIdentifier,
	account_id: AccountId,
) -> Option<Vec<OfferingData>> {
	let b_id = BusinessIdentifier::new(cid, account_id);
	api.client()
		.request("encointer_bazaarGetOfferingsForBusiness", rpc_params![b_id])
		.await
		.expect("Could not find any business offerings...")
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

async fn get_all_balances(
	api: &Api,
	account_id: &AccountId,
) -> Option<Vec<(CommunityIdentifier, BalanceEntry<BlockNumber>)>> {
	api.client()
		.request("encointer_getAllBalances", rpc_params![account_id])
		.await
		.expect("Could not query all balances...")
}

async fn get_asset_fee_details(
	api: &Api,
	cid_str: &str,
	encoded_xt: &Bytes,
) -> Option<FeeDetails<NumberOrHex>> {
	let cid = verify_cid(api, cid_str, None).await;

	api.client()
		.request("encointer_queryAssetFeeDetails", rpc_params![cid, encoded_xt])
		.await
		.expect("Could not query asset fee details")
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

fn apply_demurrage(
	entry: BalanceEntry<BlockNumber>,
	current_block: BlockNumber,
	demurrage_per_block: Demurrage,
) -> BalanceType {
	let elapsed_time_block_number = current_block.checked_sub(entry.last_update).unwrap();
	let elapsed_time_u32: u32 = elapsed_time_block_number;
	let elapsed_time = Demurrage::from_num(elapsed_time_u32);
	let exponent = -demurrage_per_block * elapsed_time;
	debug!(
		"demurrage per block {}, current_block {}, last {}, elapsed_blocks {}",
		demurrage_per_block, current_block, entry.last_update, elapsed_time
	);
	let exp_result = exp(exponent).unwrap();
	entry.principal.checked_mul(to_U64F64(exp_result).unwrap()).unwrap()
}

async fn send_bazaar_xt(matches: &ArgMatches<'_>, bazaar_call: &BazaarCalls) -> Result<(), ()> {
	let business_owner = matches.account_arg().map(get_pair_from_str).unwrap();

	let mut api = get_chain_api(matches).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(
		business_owner.clone(),
	)));
	let cid =
		verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
	let ipfs_cid = matches.ipfs_cid_arg().expect("ipfs cid needed");

	let tx_payment_cid_arg = matches.tx_payment_cid_arg();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
	let xt: EncointerXt<_> =
		compose_extrinsic!(api, "EncointerBazaar", &bazaar_call.to_string(), cid, ipfs_cid)
			.unwrap();
	ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
	// send and watch extrinsic until ready
	let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
	println!(
		"{} for {}. xt-status: '{:?}'",
		bazaar_call.to_string(),
		business_owner.public(),
		report.status
	);
	Ok(())
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
		let remaining_tickets = total_newbie_tickets
			- api
				.get_storage_double_map(
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BazaarCalls {
	CreateBusiness,
	UpdateBusiness,
	CreateOffering,
}

impl ToString for BazaarCalls {
	fn to_string(&self) -> String {
		match self {
			BazaarCalls::CreateBusiness => "create_business".to_string(),
			BazaarCalls::UpdateBusiness => "update_business".to_string(),
			BazaarCalls::CreateOffering => "create_offering".to_string(),
		}
	}
}

async fn set_api_extrisic_params_builder(api: &mut Api, tx_payment_cid_arg: Option<&str>) {
	let mut tx_params = CommunityCurrencyTipExtrinsicParamsBuilder::new().tip(0);
	if let Some(tx_payment_cid) = tx_payment_cid_arg {
		tx_params = tx_params.tip(
			CommunityCurrencyTip::new(0).of_community(verify_cid(api, tx_payment_cid, None).await),
		);
	}
	let _ = &api.set_additional_params(tx_params);
}
