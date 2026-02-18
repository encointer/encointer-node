use crate::{
	cli::Cli,
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
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, ParentchainExtrinsicSigner, SchedulerApi,
};
use encointer_primitives::communities::{CommunityIdentifier, GeoHash, Location};

use crate::{
	community_spec::remove_location_call,
	utils::{send_and_wait_for_finalized, BatchCall, CallWrapping},
};
use encointer_primitives::scheduler::CeremonyPhaseType;
use itertools::Itertools;
use log::{error, info, warn};
use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::Ss58Codec;
use sp_core::Pair;
use sp_keyring::Sr25519Keyring as AccountKeyring;
use std::str::FromStr;
use substrate_api_client::ac_node_api::Metadata;

pub async fn new_community(
	cli: &Cli,
	spec_file: &str,
	signer_arg: Option<&str>,
	dryrun: bool,
	wrap_call: &str,
	batch_size: u32,
) {
	// -----setup
	let spec = read_community_spec_from_file(spec_file);
	let cid = spec.community_identifier();

	let signer = signer_arg
		.map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());
	let signer = ParentchainExtrinsicSigner::new(signer);

	let mut api = get_chain_api(cli).await;
	api.set_signer(signer);

	// ------- create calls for xt's
	let new_community_call = OpaqueCall::from_tuple(&new_community_call(&spec, api.metadata()));
	// only the first meetup location has been registered now. register all others one-by-one
	let add_location_batch_calls =
		create_add_location_batches(api.metadata(), spec.locations(), cid, batch_size);

	let call_wrapping = CallWrapping::from_str(wrap_call).unwrap_or(CallWrapping::None);
	info!("XT call wrapping: {:?}", call_wrapping);

	let (new_community_final_call, add_location_batch_final_call) = match call_wrapping {
		CallWrapping::None => (
			new_community_call,
			add_location_batch_calls
				.into_iter()
				.map(|c| OpaqueCall::from_tuple(&c))
				.collect::<Vec<_>>(),
		),
		CallWrapping::Sudo => {
			if !contains_sudo_pallet(api.metadata()) {
				panic!("Want to wrap call with sudo, but sudo does not exist on this chain.");
			}

			let sudo_new_community = sudo_call(api.metadata(), new_community_call);
			let sudo_add_location_batch = add_location_batch_calls
				.into_iter()
				.map(|call| sudo_call(api.metadata(), call))
				.collect::<Vec<_>>();
			info!("Printing raw sudo calls for js/apps for cid: {}", cid);
			print_raw_call("sudo(new_community)", &sudo_new_community);

			for call in sudo_add_location_batch.iter() {
				print_raw_call("sudo(utility_batch(add_location))", &call);
			}

			let opaque_sudo_add_location = sudo_add_location_batch
				.into_iter()
				.map(|call| OpaqueCall::from_tuple(&call))
				.collect();

			(OpaqueCall::from_tuple(&sudo_new_community), opaque_sudo_add_location)
		},
		CallWrapping::Collective => {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!(
				"Printing raw collective propose calls with threshold {} for js/apps for cid: {}",
				threshold, cid
			);
			let propose_new_community =
				collective_propose_call(api.metadata(), threshold, new_community_call);
			print_raw_call("collective_propose(new_community)", &propose_new_community);

			let propose_add_location_batch = add_location_batch_calls
				.into_iter()
				.map(|call| collective_propose_call(api.metadata(), threshold, call))
				.collect::<Vec<_>>();

			for call in propose_add_location_batch.iter() {
				print_raw_call("collective_propose(utility_batch(add_location))", &call);
			}

			let opaque_collective_add_location = propose_add_location_batch
				.into_iter()
				.map(|call| OpaqueCall::from_tuple(&call))
				.collect();

			(OpaqueCall::from_tuple(&propose_new_community), opaque_collective_add_location)
		},
	};

	if !dryrun {
		info!("Sending transactions");
	} else {
		info!("skipping sending transactions");
		return;
	}

	// ---- send xt's to chain
	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

	send_and_wait_for_finalized(&api, xt(&api, new_community_final_call).await, tx_payment_cid_arg)
		.await;
	println!("{cid}");

	if api.get_current_phase(None).await.unwrap() != CeremonyPhaseType::Registering {
		error!("Wrong ceremony phase for registering new locations for {}", cid);
		error!("Aborting without registering additional locations");
		std::process::exit(exit_code::WRONG_PHASE);
	}

	for call in add_location_batch_final_call {
		send_and_wait_for_finalized(&api, xt(&api, call).await, tx_payment_cid_arg).await;
	}
}

pub async fn add_locations(cli: &Cli, spec_file: &str, signer_arg: Option<&str>, dryrun: bool) {
	// -----setup
	let spec = read_community_spec_from_file(spec_file);

	let mut api = get_chain_api(cli).await;
	if !dryrun {
		let signer = signer_arg.map_or_else(
			|| AccountKeyring::Alice.pair(),
			|signer| get_pair_from_str(signer).into(),
		);
		info!("signer ss58 is {}", signer.public().to_ss58check());
		let signer = ParentchainExtrinsicSigner::new(signer);
		api.set_signer(signer);
	}

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();

	let cid = api.verify_cid(cli.cid.as_deref().unwrap(), None).await;

	let add_location_calls: Vec<AddLocationCall> = spec
		.locations()
		.into_iter()
		.map(|l| {
			info!("adding location {:?}", l);
			add_location_call(api.metadata(), cid, l)
		})
		.collect();

	let mut add_location_maybe_batch_call = match add_location_calls.as_slice() {
		[call] => OpaqueCall::from_tuple(call),
		_ => OpaqueCall::from_tuple(&batch_call(api.metadata(), add_location_calls.clone())),
	};

	if signer_arg.is_none() {
		// return calls as `OpaqueCall`s to get the same return type in both branches
		add_location_maybe_batch_call = if contains_sudo_pallet(api.metadata()) {
			let sudo_add_location_batch = sudo_call(api.metadata(), add_location_maybe_batch_call);
			info!("Printing raw sudo calls for js/apps for cid: {}", cid);
			print_raw_call("sudo(utility_batch(add_location))", &sudo_add_location_batch);
			OpaqueCall::from_tuple(&sudo_add_location_batch)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!(
				"Printing raw collective propose calls with threshold {} for js/apps for cid: {}",
				threshold, cid
			);
			let propose_add_location_batch =
				collective_propose_call(api.metadata(), threshold, add_location_maybe_batch_call);
			print_raw_call(
				"collective_propose(utility_batch(add_location))",
				&propose_add_location_batch,
			);
			OpaqueCall::from_tuple(&propose_add_location_batch)
		};
	}

	if dryrun {
		println!("0x{}", hex::encode(add_location_maybe_batch_call.encode()));
	} else {
		// ---- send xt's to chain
		if api.get_current_phase(None).await.unwrap() != CeremonyPhaseType::Registering {
			error!("Wrong ceremony phase for registering new locations for {}", cid);
			error!("Aborting without registering additional locations");
			std::process::exit(exit_code::WRONG_PHASE);
		}
		send_and_wait_for_in_block(
			&api,
			xt(&api, add_location_maybe_batch_call).await,
			tx_payment_cid_arg,
		)
		.await;
	}
}

pub async fn remove_locations(
	cli: &Cli,
	signer_arg: Option<&str>,
	dryrun: bool,
	geohash: Option<&str>,
	location_index: Option<u32>,
) {
	// -----setup

	let mut api = get_chain_api(cli).await;
	if !dryrun {
		let signer = signer_arg.map_or_else(
			|| AccountKeyring::Alice.pair(),
			|signer| get_pair_from_str(signer).into(),
		);
		info!("signer ss58 is {}", signer.public().to_ss58check());
		let signer = ParentchainExtrinsicSigner::new(signer);
		api.set_signer(signer);
	}

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();

	let cid = api.verify_cid(cli.cid.as_deref().unwrap(), None).await;
	let geohash = geohash.expect("need geohash");
	let geohash = GeoHash::try_from(geohash).expect("invalid geohash");
	let location_index = location_index.expect("need location");
	let locations = api.get_locations_by_geohash(cid, geohash, None).await.unwrap();

	let mut remove_location_call = OpaqueCall::from_tuple(&remove_location_call(
		api.metadata(),
		cid,
		locations[location_index as usize],
	));

	if signer_arg.is_none() {
		// return calls as `OpaqueCall`s to get the same return type in both branches
		remove_location_call = if contains_sudo_pallet(api.metadata()) {
			let sudo_add_location_batch = sudo_call(api.metadata(), remove_location_call);
			info!("Printing raw sudo calls for js/apps for cid: {}", cid);
			print_raw_call("sudo(remove_location)", &sudo_add_location_batch);
			OpaqueCall::from_tuple(&sudo_add_location_batch)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!(
				"Printing raw collective propose calls with threshold {} for js/apps for cid: {}",
				threshold, cid
			);
			let propose_add_location_batch =
				collective_propose_call(api.metadata(), threshold, remove_location_call);
			print_raw_call("collective_propose(remove_location)", &propose_add_location_batch);
			OpaqueCall::from_tuple(&propose_add_location_batch)
		};
	}

	if dryrun {
		println!("0x{}", hex::encode(remove_location_call.encode()));
	} else {
		// ---- send xt's to chain
		if api.get_current_phase(None).await.unwrap() != CeremonyPhaseType::Registering {
			error!("Wrong ceremony phase for registering new locations for {}", cid);
			error!("Aborting without registering additional locations");
			std::process::exit(exit_code::WRONG_PHASE);
		}
		send_and_wait_for_in_block(&api, xt(&api, remove_location_call).await, tx_payment_cid_arg)
			.await;
	}
}

pub async fn list_communities(cli: &Cli) {
	let api = get_chain_api(cli).await;
	let maybe_at = cli.at_block();
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
}

pub async fn list_locations(cli: &Cli) {
	let api = get_chain_api(cli).await;
	let maybe_at = cli.at_block();
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), maybe_at)
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
