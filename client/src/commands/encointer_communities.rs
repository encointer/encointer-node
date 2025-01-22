use crate::{
	cli_args::EncointerArgsExtractor,
	community_spec::{
		add_location_call, new_community_call, read_community_spec_from_file, AddLocationCall,
		CommunitySpec,
	},
	exit_code,
	utils::{
		batch_call, collective_propose_call, contains_sudo_pallet, get_chain_api, get_councillors,
		keys::get_pair_from_str, print_raw_call, send_and_wait_for_in_block, sudo_call, xt,
		OpaqueCall,
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, ParentchainExtrinsicSigner, SchedulerApi,
};
use encointer_primitives::communities::{CommunityIdentifier, Location};

use crate::utils::{send_and_wait_for_finalized, BatchCall, CallWrapping};
use encointer_primitives::scheduler::CeremonyPhaseType;
use itertools::Itertools;
use log::{error, info, warn};
use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::Ss58Codec;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use substrate_api_client::ac_node_api::Metadata;

pub fn new_community(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
        // -----setup
        let spec_file = matches.value_of("specfile").unwrap();
        let spec = read_community_spec_from_file(spec_file);
        let cid = spec.community_identifier();

        let signer = matches.signer_arg()
            .map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());
        let signer = ParentchainExtrinsicSigner::new(signer);

        let mut api = get_chain_api(matches).await;
        api.set_signer(signer);


        // ------- create calls for xt's
        let new_community_call = OpaqueCall::from_tuple(&new_community_call(&spec, api.metadata()));
        // only the first meetup location has been registered now. register all others one-by-one
        let add_location_batch_calls = create_add_location_batches(api.metadata(), spec.locations(), cid, matches.batch_size_arg());

        info!("XT call wrapping: {:?}", matches.wrap_call_arg());

        let (new_community_final_call, add_location_batch_final_call) = match matches.wrap_call_arg() {
            CallWrapping::None => {
                (new_community_call, add_location_batch_calls.into_iter().map(|c| OpaqueCall::from_tuple(&c)).collect::<Vec<_>>())
            }
            CallWrapping::Sudo => {
                if !contains_sudo_pallet(api.metadata()) {
                    panic!("Want to wrap call with sudo, but sudo does not exist on this chain.");
                }

                let sudo_new_community = sudo_call(api.metadata(), new_community_call);
                let sudo_add_location_batch = add_location_batch_calls.into_iter().map(|call| sudo_call(api.metadata(), call)).collect::<Vec<_>>();
                info!("Printing raw sudo calls for js/apps for cid: {}", cid);
                print_raw_call("sudo(new_community)", &sudo_new_community);

                for call in sudo_add_location_batch.iter() {
                    print_raw_call("sudo(utility_batch(add_location))", &call);
                }

                let opaque_sudo_add_location = sudo_add_location_batch.into_iter().map(|call| OpaqueCall::from_tuple(&call)).collect();

                (OpaqueCall::from_tuple(&sudo_new_community), opaque_sudo_add_location)
            }
            CallWrapping::Collective => {
                let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
                info!("Printing raw collective propose calls with threshold {} for js/apps for cid: {}", threshold, cid);
                let propose_new_community = collective_propose_call(api.metadata(), threshold, new_community_call);
                print_raw_call("collective_propose(new_community)", &propose_new_community);


                let propose_add_location_batch = add_location_batch_calls
                    .into_iter()
                    .map(|call| collective_propose_call(api.metadata(), threshold, call)).collect::<Vec<_>>();

                for call in propose_add_location_batch.iter() {
                    print_raw_call("collective_propose(utility_batch(add_location))", &call);
                }

                let opaque_collective_add_location = propose_add_location_batch.into_iter().map(|call| OpaqueCall::from_tuple(&call)).collect();

                (OpaqueCall::from_tuple(&propose_new_community), opaque_collective_add_location)
            }
        };

        if !matches.dryrun_flag() {
            info!("Sending transactions");
        } else {
            info!("skipping sending transactions");
            return Ok(());
        }

        // ---- send xt's to chain
        let tx_payment_cid_arg = matches.tx_payment_cid_arg();
        set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

        send_and_wait_for_finalized(&api, xt(&api, new_community_final_call).await, matches.tx_payment_cid_arg()).await;
        println!("{cid}");

        if api.get_current_phase(None).await.unwrap() != CeremonyPhaseType::Registering {
            error!("Wrong ceremony phase for registering new locations for {}", cid);
            error!("Aborting without registering additional locations");
            std::process::exit(exit_code::WRONG_PHASE);
        }

        for call in add_location_batch_final_call {
            send_and_wait_for_finalized(&api, xt(&api, call).await, tx_payment_cid_arg).await;
        }
        Ok(())
    })
        .into()
}
pub fn add_locations(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
        // -----setup
        let spec_file = matches.value_of("specfile").unwrap();
        let spec = read_community_spec_from_file(spec_file);

        let mut api = get_chain_api(matches).await;
        if !matches.dryrun_flag() {
            let signer = matches.signer_arg()
                .map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());
            info!("signer ss58 is {}", signer.public().to_ss58check());
            let signer = ParentchainExtrinsicSigner::new(signer);
            api.set_signer(signer);
        }

        let tx_payment_cid_arg = matches.tx_payment_cid_arg();

        let cid = api.verify_cid(matches.cid_arg().unwrap(), None).await;

        let add_location_calls: Vec<AddLocationCall> = spec.locations().into_iter().map(|l|
            {
                info!("adding location {:?}", l);
                add_location_call(api.metadata(), cid, l)
            }
        ).collect();

        let mut add_location_maybe_batch_call = match add_location_calls.as_slice() {
            [call] => OpaqueCall::from_tuple(call),
            _ => OpaqueCall::from_tuple(&batch_call(api.metadata(), add_location_calls.clone()))
        };

        if matches.signer_arg().is_none() {
            // return calls as `OpaqueCall`s to get the same return type in both branches
            add_location_maybe_batch_call = if contains_sudo_pallet(api.metadata()) {
                let sudo_add_location_batch = sudo_call(api.metadata(), add_location_maybe_batch_call);
                info!("Printing raw sudo calls for js/apps for cid: {}", cid);
                print_raw_call("sudo(utility_batch(add_location))", &sudo_add_location_batch);
                OpaqueCall::from_tuple(&sudo_add_location_batch)
            } else {
                let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
                info!("Printing raw collective propose calls with threshold {} for js/apps for cid: {}", threshold, cid);
                let propose_add_location_batch = collective_propose_call(api.metadata(), threshold, add_location_maybe_batch_call);
                print_raw_call("collective_propose(utility_batch(add_location))", &propose_add_location_batch);
                OpaqueCall::from_tuple(&propose_add_location_batch)
            };
        }

        if matches.dryrun_flag() {
            println!("0x{}", hex::encode(add_location_maybe_batch_call.encode()));
        } else {
            // ---- send xt's to chain
            if api.get_current_phase(None).await.unwrap() != CeremonyPhaseType::Registering {
                error!("Wrong ceremony phase for registering new locations for {}", cid);
                error!("Aborting without registering additional locations");
                std::process::exit(exit_code::WRONG_PHASE);
            }
            send_and_wait_for_in_block(&api, xt(&api, add_location_maybe_batch_call).await, tx_payment_cid_arg).await;
        }
        Ok(())
    })
        .into()
}
pub fn list_communities(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
        let api = get_chain_api(matches).await;
        let maybe_at = matches.at_block_arg();
        if maybe_at.is_some() {
            warn!("fetching community names doesn't support --at. will fetch current communities and apply --at to values")
        }
        let names = api.get_cid_names().await.unwrap();
        println!("number of communities:  {}", names.len());
        for n in names.iter() {
            let loc = api.get_locations(n.cid).await.unwrap();
            let cii = api.get_nominal_income(n.cid, maybe_at).await.unwrap_or_default();
            let demurrage = api.get_demurrage_per_block(n.cid, maybe_at).await.unwrap_or_default();
            let meta = api.get_community_metadata(n.cid, maybe_at).await.unwrap_or_default();
            println!(
                "{}: {}, locations: {}, nominal income: {} {}, demurrage: {:?}/block, {:?}",
                n.cid,
                String::from_utf8(n.name.to_vec()).unwrap(),
                loc.len(),
                cii,
                String::from_utf8_lossy(&meta.symbol),
                demurrage,
                meta.rules
            );
        }
        Ok(())
    })
        .into()
}
pub fn list_locations(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();
		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), maybe_at)
			.await;
		println!("listing locations for cid {cid}");
		let loc = api.get_locations(cid).await.unwrap();
		for l in loc.iter() {
			println!(
				"lat: {} lon: {} (raw lat: {} lon: {})",
				l.lat,
				l.lon,
				i128::decode(&mut l.lat.encode().as_slice()).unwrap(),
				i128::decode(&mut l.lon.encode().as_slice()).unwrap()
			);
		}
		Ok(())
	})
	.into()
}

fn create_add_location_batches(
	metadata: &Metadata,
	locations: Vec<Location>,
	cid: CommunityIdentifier,
	batch_size: u32,
) -> Vec<BatchCall<AddLocationCall>> {
	info!("Creating add location batches of size: {:?}", batch_size);

	locations
		.into_iter()
		.skip(1) // Skip the first location
		.map(|l| add_location_call(metadata, cid, l))
		.chunks(batch_size as usize)
		.into_iter()
		.map(|chunk| chunk.collect())
		.map(|b| batch_call(metadata, b))
		.collect() // Collect all batches into a Vec of BatchCall
}
