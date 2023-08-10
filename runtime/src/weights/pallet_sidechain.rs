//! Autogenerated weights for `pallet_teerex`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-11-11, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("integritee-solo-fresh"), DB CACHE: 128

// Executed Command:
// target/release/integritee-node
// benchmark
// --chain=integritee-solo-fresh
// --steps=50
// --repeat=20
// --pallet=pallet_teerex
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=runtime/src/weights/pallet_teerex.rs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_sidechain.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_sidechain::WeightInfo for WeightInfo<T> {
    // Storage: Teerex EnclaveIndex (r:1 w:0)
    // Storage: Teerex EnclaveRegistry (r:1 w:0)
    // Storage: Teerex WorkerForShard (r:0 w:1)
    fn confirm_imported_sidechain_block() -> Weight {
        Weight::from_parts(70_298_000,0u64 )
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}
