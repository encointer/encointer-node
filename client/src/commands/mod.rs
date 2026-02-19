pub mod encointer_bazaar;
pub mod encointer_ceremonies;
pub mod encointer_communities;
pub mod encointer_core;
pub mod encointer_democracy;
pub mod encointer_faucet;
pub mod encointer_ipfs;
pub mod encointer_offline_payment;
pub mod encointer_reputation_commitments;
pub mod encointer_reputation_rings;
pub mod encointer_scheduler;
pub mod encointer_treasuries;
pub mod frame;
pub mod keystore;

use crate::cli::*;

pub async fn run(cli: &Cli) {
	match &cli.command {
		Commands::Chain(cmd) => match cmd {
			ChainCmd::Balance { account, all } => encointer_core::balance(cli, account, *all).await,
			ChainCmd::Transfer { from, to, amount, dryrun } =>
				encointer_core::transfer(cli, from, to, amount, *dryrun).await,
			ChainCmd::TransferAll { from, to } => encointer_core::transfer_all(cli, from, to).await,
			ChainCmd::Listen { events, blocks } =>
				encointer_core::listen(cli, *events, *blocks).await,
			ChainCmd::PrintMetadata => frame::print_metadata(cli).await,
		},
		Commands::Account { cmd } => match cmd {
			AccountCmd::New { seed } => keystore::new_account(seed.as_deref()),
			AccountCmd::List => keystore::list_accounts(),
			AccountCmd::Export { account } => keystore::export_secret(account),
			AccountCmd::Fund { fundees } => frame::fund(cli, fundees).await,
			AccountCmd::PoseidonCommitment { cmd } => match cmd {
				PoseidonCommitmentCmd::Register { account } =>
					encointer_offline_payment::register_offline_identity(cli, account).await,
				PoseidonCommitmentCmd::Get { account } =>
					encointer_offline_payment::get_offline_identity(cli, account).await,
			},
			AccountCmd::BandersnatchPubkey { cmd } => match cmd {
				BandersnatchPubkeyCmd::Register { account, key } =>
					encointer_reputation_rings::register_bandersnatch_key(
						cli,
						account,
						key.as_deref(),
					)
					.await,
			},
		},
		Commands::Community { cmd } => match cmd {
			CommunityCmd::New { specfile, signer, dryrun, wrap_call, batch_size } =>
				encointer_communities::new_community(
					cli,
					specfile,
					signer.as_deref(),
					*dryrun,
					wrap_call,
					*batch_size,
				)
				.await,
			CommunityCmd::List => encointer_communities::list_communities(cli).await,
			CommunityCmd::Issuance => encointer_core::issuance(cli).await,
			CommunityCmd::Location { cmd } => match cmd {
				LocationCmd::List => encointer_communities::list_locations(cli).await,
				LocationCmd::Add { specfile, signer, dryrun } =>
					encointer_communities::add_locations(cli, specfile, signer.as_deref(), *dryrun)
						.await,
				LocationCmd::Remove { signer, dryrun, geohash, location_index } =>
					encointer_communities::remove_locations(
						cli,
						signer.as_deref(),
						*dryrun,
						geohash.as_deref(),
						*location_index,
					)
					.await,
			},
			CommunityCmd::Treasury { cmd } => match cmd {
				TreasuryCmd::GetAccount => encointer_treasuries::get_treasury_account(cli).await,
				TreasuryCmd::SwapOption { cmd } => match cmd {
					SwapOptionCmd::GetNative { account } =>
						encointer_treasuries::get_swap_native_option(cli, account).await,
					SwapOptionCmd::GetAsset { account } =>
						encointer_treasuries::get_swap_asset_option(cli, account).await,
					SwapOptionCmd::ExerciseNative { account, amount } =>
						encointer_treasuries::swap_native(cli, account, *amount).await,
					SwapOptionCmd::ExerciseAsset { account, amount } =>
						encointer_treasuries::swap_asset(cli, account, *amount).await,
				},
			},
		},
		Commands::Ceremony { cmd } => match cmd {
			CeremonyCmd::Phase => encointer_scheduler::get_phase(cli).await,
			CeremonyCmd::Index => encointer_scheduler::get_cindex(cli).await,
			CeremonyCmd::Participant { cmd } => match cmd {
				ParticipantCmd::Register { account, signer } =>
					encointer_ceremonies::register_participant(cli, account, signer.as_deref())
						.await,
				ParticipantCmd::Unregister { account, signer, ceremony_index } =>
					encointer_ceremonies::unregister_participant(
						cli,
						account,
						signer.as_deref(),
						*ceremony_index,
					)
					.await,
				ParticipantCmd::Upgrade { account, signer } =>
					encointer_ceremonies::upgrade_registration(cli, account, signer.as_deref())
						.await,
				ParticipantCmd::Endorse { bootstrapper, endorsees } =>
					encointer_ceremonies::endorse(cli, bootstrapper, endorsees).await,
				ParticipantCmd::Attest { account, attestees } =>
					encointer_ceremonies::attest_attendees(cli, account, attestees).await,
				ParticipantCmd::NewClaim { account, vote } =>
					encointer_ceremonies::new_claim(cli, account, *vote).await,
				ParticipantCmd::ClaimReward { signer, meetup_index, all } =>
					encointer_ceremonies::claim_reward(cli, signer.as_deref(), *meetup_index, *all)
						.await,
				ParticipantCmd::List { ceremony_index } =>
					encointer_ceremonies::list_participants(cli, *ceremony_index).await,
				ParticipantCmd::Reputation { account } =>
					encointer_ceremonies::reputation(cli, account).await,
				ParticipantCmd::ProofOfAttendance { account, ceremony_index } =>
					encointer_ceremonies::get_proof_of_attendance(cli, account, *ceremony_index)
						.await,
			},
			CeremonyCmd::ListMeetups { ceremony_index } =>
				encointer_ceremonies::list_meetups(cli, *ceremony_index).await,
			CeremonyCmd::ListAttestees { ceremony_index } =>
				encointer_ceremonies::list_attestees(cli, *ceremony_index).await,
			CeremonyCmd::ListReputables => encointer_ceremonies::list_reputables(cli).await,
			CeremonyCmd::Stats { ceremony_index } =>
				encointer_ceremonies::print_ceremony_stats(cli, *ceremony_index).await,
			CeremonyCmd::Admin { cmd } => match cmd {
				CeremonyAdminCmd::NextPhase { signer } =>
					encointer_scheduler::next_phase(cli, signer.as_deref()).await,
				CeremonyAdminCmd::BootstrapperTickets =>
					encointer_ceremonies::bootstrappers_with_remaining_newbie_tickets(cli).await,
				CeremonyAdminCmd::Purge { from_cindex, to_cindex } =>
					encointer_ceremonies::purge_community_ceremony(cli, *from_cindex, *to_cindex)
						.await,
				CeremonyAdminCmd::SetMeetupTimeOffset { time_offset } =>
					encointer_ceremonies::set_meetup_time_offset(cli, *time_offset).await,
			},
		},
		Commands::Democracy { cmd } => match cmd {
			DemocracyCmd::Propose { cmd } => match cmd {
				ProposeCmd::SetInactivityTimeout { account, inactivity_timeout } =>
					encointer_democracy::submit_set_inactivity_timeout_proposal(
						cli,
						account,
						*inactivity_timeout,
					)
					.await,
				ProposeCmd::UpdateNominalIncome { account, nominal_income } =>
					encointer_democracy::submit_update_nominal_income_proposal(
						cli,
						account,
						*nominal_income,
					)
					.await,
				ProposeCmd::UpdateDemurrage { account, demurrage_halving_blocks } =>
					encointer_democracy::submit_update_demurrage_proposal(
						cli,
						account,
						*demurrage_halving_blocks,
					)
					.await,
				ProposeCmd::Petition { account, demand } =>
					encointer_democracy::submit_petition(cli, account, demand).await,
				ProposeCmd::SpendNative { account, to, amount } =>
					encointer_democracy::submit_spend_native_proposal(cli, account, to, *amount)
						.await,
				ProposeCmd::IssueSwapNativeOption {
					account,
					to,
					native_allowance,
					rate,
					do_burn,
					valid_from,
					valid_until,
				} =>
					encointer_democracy::submit_issue_swap_native_option_proposal(
						cli,
						account,
						to,
						*native_allowance,
						*rate,
						*do_burn,
						*valid_from,
						*valid_until,
					)
					.await,
				ProposeCmd::IssueSwapAssetOption {
					account,
					to,
					asset_id,
					asset_allowance,
					rate,
					do_burn,
					valid_from,
					valid_until,
				} =>
					encointer_democracy::submit_issue_swap_asset_option_proposal(
						cli,
						account,
						to,
						asset_id,
						*asset_allowance,
						*rate,
						*do_burn,
						*valid_from,
						*valid_until,
					)
					.await,
			},
			DemocracyCmd::Proposal { cmd } => match cmd {
				ProposalCmd::List { all } => encointer_democracy::list_proposals(cli, *all).await,
				ProposalCmd::UpdateState { account, proposal_id } =>
					encointer_democracy::update_proposal_state(cli, account, *proposal_id).await,
			},
			DemocracyCmd::EnactmentQueue => encointer_democracy::list_enactment_queue(cli).await,
			DemocracyCmd::Vote { account, proposal_id, vote, reputation_vec } =>
				encointer_democracy::vote(cli, account, *proposal_id, vote, reputation_vec).await,
		},
		Commands::Bazaar { cmd } => match cmd {
			BazaarCmd::Business { cmd } => match cmd {
				BusinessCmd::Create { account, ipfs_cid } =>
					encointer_bazaar::create_business(cli, account, ipfs_cid).await,
				BusinessCmd::Update { account, ipfs_cid } =>
					encointer_bazaar::update_business(cli, account, ipfs_cid).await,
				BusinessCmd::List => encointer_bazaar::list_businesses(cli).await,
				BusinessCmd::Offerings { account } =>
					encointer_bazaar::list_business_offerings(cli, account).await,
			},
			BazaarCmd::Offering { cmd } => match cmd {
				OfferingCmd::Create { account, ipfs_cid } =>
					encointer_bazaar::create_offering(cli, account, ipfs_cid).await,
				OfferingCmd::List => encointer_bazaar::list_offerings(cli).await,
			},
		},
		Commands::Faucet { cmd } => match cmd {
			FaucetCmd::Create {
				account,
				faucet_name,
				faucet_balance,
				faucet_drip_amount,
				whitelist,
			} =>
				encointer_faucet::create_faucet(
					cli,
					account,
					faucet_name,
					*faucet_balance,
					*faucet_drip_amount,
					whitelist,
				)
				.await,
			FaucetCmd::Drip { account, faucet_account, cindex } =>
				encointer_faucet::drip_faucet(cli, account, faucet_account, *cindex).await,
			FaucetCmd::Dissolve { signer, faucet_account, faucet_beneficiary } =>
				encointer_faucet::dissolve_faucet(
					cli,
					signer.as_deref(),
					faucet_account,
					faucet_beneficiary,
				)
				.await,
			FaucetCmd::Close { account, faucet_account } =>
				encointer_faucet::close_faucet(cli, account, faucet_account).await,
			FaucetCmd::SetReserveAmount { signer, faucet_reserve_amount } =>
				encointer_faucet::set_faucet_reserve_amount(
					cli,
					signer.as_deref(),
					*faucet_reserve_amount,
				)
				.await,
			FaucetCmd::List => encointer_faucet::list_faucets(cli).await,
		},
		Commands::Personhood { cmd } => match cmd {
			PersonhoodCmd::Ring { cmd } => match cmd {
				RingCmd::Initiate { account, ceremony_index } =>
					encointer_reputation_rings::initiate_rings(cli, account, *ceremony_index).await,
				RingCmd::Continue { account } =>
					encointer_reputation_rings::continue_ring_computation(cli, account).await,
				RingCmd::Get { ceremony_index } =>
					encointer_reputation_rings::get_rings(cli, *ceremony_index).await,
			},
			PersonhoodCmd::ProveRingMembership { account, ceremony_index, level, sub_ring } =>
				encointer_reputation_rings::prove_personhood(
					cli,
					account,
					*ceremony_index,
					*level,
					*sub_ring,
				)
				.await,
			PersonhoodCmd::VerifyRingMembership { signature, ceremony_index, level, sub_ring } =>
				encointer_reputation_rings::verify_personhood(
					cli,
					signature,
					*ceremony_index,
					*level,
					*sub_ring,
				)
				.await,
			PersonhoodCmd::Commitment { cmd } => match cmd {
				CommitmentCmd::List { purpose_id } =>
					encointer_reputation_commitments::list_commitments(cli, *purpose_id).await,
				CommitmentCmd::Purposes =>
					encointer_reputation_commitments::list_purposes(cli).await,
			},
		},
		Commands::OfflinePayment { cmd } => match cmd {
			OfflinePaymentCmd::Pay { signer, to, amount, pk_file } =>
				encointer_offline_payment::generate_offline_payment(
					cli,
					signer.as_deref(),
					to,
					amount,
					pk_file.as_deref(),
				)
				.await,
			OfflinePaymentCmd::Settle {
				signer,
				proof_file,
				proof,
				sender,
				recipient,
				amount,
				nullifier,
			} =>
				encointer_offline_payment::submit_offline_payment(
					cli,
					signer.as_deref(),
					proof_file.as_deref(),
					proof.as_deref(),
					sender.as_deref(),
					recipient.as_deref(),
					amount.as_deref(),
					nullifier.as_deref(),
				)
				.await,
			OfflinePaymentCmd::Admin { cmd } => match cmd {
				OfflinePaymentAdminCmd::SetVk { signer, vk_file, vk } =>
					encointer_offline_payment::set_verification_key(
						cli,
						signer.as_deref(),
						vk_file.as_deref(),
						vk.as_deref(),
					)
					.await,
				OfflinePaymentAdminCmd::GenerateTestVk =>
					encointer_offline_payment::generate_test_vk(),
				OfflinePaymentAdminCmd::TrustedSetup { cmd } => match cmd {
					TrustedSetupCmd::Generate { pk_out, vk_out } =>
						encointer_offline_payment::generate_trusted_setup(pk_out, vk_out),
					TrustedSetupCmd::Verify { pk, vk } =>
						encointer_offline_payment::verify_trusted_setup(pk, vk),
				},
				OfflinePaymentAdminCmd::Ceremony { cmd } => match cmd {
					SetupCeremonyCmd::Init { pk_out, transcript } =>
						encointer_offline_payment::cmd_ceremony_init(pk_out, transcript),
					SetupCeremonyCmd::Contribute { pk, transcript, participant } =>
						encointer_offline_payment::cmd_ceremony_contribute(
							pk,
							transcript,
							participant,
						),
					SetupCeremonyCmd::Verify { pk, transcript } =>
						encointer_offline_payment::cmd_ceremony_verify(pk, transcript),
					SetupCeremonyCmd::Finalize { pk, pk_out, vk_out } =>
						encointer_offline_payment::cmd_ceremony_finalize(pk, pk_out, vk_out),
				},
				OfflinePaymentAdminCmd::InspectKey { file } =>
					encointer_offline_payment::inspect_setup_key(file),
			},
		},
		Commands::Ipfs { cmd } => match cmd {
			IpfsCmd::Upload { signer, gateway, file_path } =>
				encointer_ipfs::ipfs_upload(cli, signer, gateway, file_path).await,
		},
	}
}
