#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use alloc::{borrow::Cow, vec, vec::Vec};
use frame_support::{
	derive_impl,
	genesis_builder_helper::{build_state, get_preset},
	traits::{tokens::ConversionToAssetBalance, ConstU128, EqualPrivilegeOnly, InstanceFilter},
};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, ConstU32, OpaqueMetadata};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature, RuntimeDebug,
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{Contains, KeyOwnerProofSystem, Randomness, StorageInfo},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	PalletId, StorageValue,
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::FungibleAdapter;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub use pallet_encointer_balances::Call as EncointerBalancesCall;
pub use pallet_encointer_bazaar::Call as EncointerBazaarCall;
pub use pallet_encointer_ceremonies::Call as EncointerCeremoniesCall;
pub use pallet_encointer_communities::Call as EncointerCommunitiesCall;
pub use pallet_encointer_democracy::Call as EncointerDemocracyCall;
pub use pallet_encointer_faucet::Call as EncointerFaucetCall;
pub use pallet_encointer_reputation_commitments::Call as EncointerReputationCommitmentsCall;
pub use pallet_encointer_scheduler::Call as EncointerSchedulerCall;

pub use encointer_balances_tx_payment::{AssetBalanceOf, AssetIdOf, BalanceToCommunityBalance};
pub use encointer_primitives::{
	balances::{BalanceEntry, BalanceType, Demurrage},
	bazaar::{BusinessData, BusinessIdentifier, OfferingData},
	ceremonies::{AggregatedAccountData, CeremonyIndexType, CeremonyInfo, CommunityReputation},
	common::PalletString,
	communities::{CommunityIdentifier, Location},
	scheduler::CeremonyPhaseType,
};
use frame_support::traits::{
	tokens::{ConversionFromAssetBalance, PayFromAccount, PaymentStatus},
	ConstBool,
};
use frame_system::{EnsureRoot, EnsureSigned};
pub use polkadot_runtime_common::impls::VersionedLocatableAsset;
use sp_runtime::traits::IdentityLookup;

mod weights;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A type to hold UTC unix epoch [ms]
pub type Moment = u64;
pub const ONE_DAY: Moment = 86_400_000;

pub type AssetId = AssetIdOf<Runtime>;
pub type AssetBalance = AssetBalanceOf<Runtime>;

const MILLICENTS: Balance = 1_000_000_000;
const CENTS: Balance = 1_000 * MILLICENTS;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

// To learn more about runtime versioning and what each of the following value means:
//   https://docs.substrate.io/v3/runtime/upgrades#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("encointer-node-notee"),
	impl_name: Cow::Borrowed("encointer-node-notee"),
	authoring_version: 0,
	spec_version: 401,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 5,
	system_version: 0,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

//TODO add meaningful values
parameter_types! {
	pub const ProxyDepositBase: Balance = 32;
	pub const ProxyDepositFactor: Balance = 32;
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = 32;
	pub const AnnouncementDepositFactor: Balance = 32;
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	BazaarEdit,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(c, RuntimeCall::EncointerBazaar(..)),
			ProxyType::BazaarEdit => matches!(
				c,
				RuntimeCall::EncointerBazaar(EncointerBazaarCall::create_offering { .. }) |
					RuntimeCall::EncointerBazaar(EncointerBazaarCall::update_offering { .. }) |
					RuntimeCall::EncointerBazaar(EncointerBazaarCall::delete_offering { .. })
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = System;
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::with_sensible_defaults(
			(Weight::from_parts(2, 0) * WEIGHT_REF_TIME_PER_SECOND).set_proof_size(u64::MAX),
			NORMAL_DISPATCH_RATIO,
		);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// The block type.
	type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const MaxAuthorities: u32 = 100_000;
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	pub const BondingDuration: sp_staking::EraIndex = 24 * 28;
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = <() as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type EquivocationReportSystem = ();
	type WeightInfo = (); // grandpa has default non-zero implementations for `()`
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = (Aura, EncointerScheduler);
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

pub const EXISTENTIAL_DEPOSIT: u128 = 500;

parameter_types! {
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = ConstU32<128>;
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = ();
	type BlockNumberProvider = System;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const MomentsPerDay: Moment = 86_400_000; // [ms/d]
	pub const DefaultDemurrage: Demurrage = Demurrage::from_bits(0x0000000000000000000001E3F0A8A973_i128);
	/// 0.000005
	pub const EncointerExistentialDeposit: BalanceType = BalanceType::from_bits(0x0000000000000000000053e2d6238da4_u128);
	pub const MeetupSizeTarget: u64 = 15;
	pub const MeetupMinSize: u64 = 3;
	pub const MeetupNewbieLimitDivider: u64 = 2;
	pub const FaucetPalletId: PalletId = PalletId(*b"ectrfct0");
}

impl pallet_encointer_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// attention!: EncointerDemocracy must be first hook as it potentially changes the rules for following hooks
	type OnCeremonyPhaseChange = (
		pallet_encointer_democracy::Pallet<Runtime>,
		pallet_encointer_ceremonies::Pallet<Runtime>,
		pallet_encointer_reputation_rings::Pallet<Runtime>,
	);
	type MomentsPerDay = MomentsPerDay;
	type CeremonyMaster = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_encointer_scheduler::WeightInfo<Runtime>;
}

impl pallet_encointer_ceremonies::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CeremonyMaster = EnsureRoot<AccountId>;
	type Public = <MultiSignature as Verify>::Signer;
	type Signature = MultiSignature;
	// Note: in production networks it is advised to use babes randomness source.
	// But we have low security requirements here, so it should be fine.
	type RandomnessSource = pallet_insecure_randomness_collective_flip::Pallet<Runtime>;
	type MeetupSizeTarget = MeetupSizeTarget;
	type MeetupMinSize = MeetupMinSize;
	type MeetupNewbieLimitDivider = MeetupNewbieLimitDivider;
	type WeightInfo = weights::pallet_encointer_ceremonies::WeightInfo<Runtime>;
	type MaxAttestations = ConstU32<100>;
}

impl pallet_encointer_communities::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CommunityMaster = EnsureRoot<AccountId>;
	type TrustableForNonDestructiveAction = EnsureSigned<AccountId>;
	type WeightInfo = weights::pallet_encointer_communities::WeightInfo<Runtime>;
	type MaxCommunityIdentifiers = ConstU32<10000>;
	type MaxBootstrappers = ConstU32<10000>;
	type MaxLocationsPerGeohash = ConstU32<10000>;
	type MaxCommunityIdentifiersPerGeohash = ConstU32<10000>;
}

impl pallet_encointer_balances::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DefaultDemurrage = DefaultDemurrage;
	type ExistentialDeposit = EncointerExistentialDeposit;
	type WeightInfo = weights::pallet_encointer_balances::WeightInfo<Runtime>;
	type CeremonyMaster = EnsureRoot<AccountId>;
}

impl pallet_encointer_bazaar::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_encointer_bazaar::WeightInfo<Runtime>;
}

pub struct AssetTxBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl
	pallet_asset_tx_payment::BenchmarkHelperTrait<
		AccountId,
		CommunityIdentifier,
		CommunityIdentifier,
	> for AssetTxBenchmarkHelper
{
	fn create_asset_id_parameter(_id: u32) -> (CommunityIdentifier, CommunityIdentifier) {
		Default::default()
	}

	fn setup_balances_and_pool(asset_id: CommunityIdentifier, account: AccountId) {
		use frame_support::traits::fungible::Mutate;
		Balances::set_balance(&account, encointer_balances_tx_payment::ONE_KSM);
		EncointerBalances::issue(asset_id, &account, BalanceType::from_num(100u32)).unwrap();
	}
}

impl pallet_asset_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = pallet_encointer_balances::Pallet<Runtime>;
	type OnChargeAssetTransaction = pallet_asset_tx_payment::FungiblesAdapter<
		encointer_balances_tx_payment::BalanceToCommunityBalance<Runtime>,
		encointer_balances_tx_payment::BurnCredit,
	>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetTxBenchmarkHelper;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * MILLICENTS;
	pub const ProposalBondMaximum: Balance = 500 * CENTS;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const PayoutSpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaxApprovals: u32 = 10;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub struct NoConversion;
impl ConversionFromAssetBalance<u128, (), u128> for NoConversion {
	type Error = ();
	fn from_asset_balance(balance: Balance, _asset_id: ()) -> Result<Balance, Self::Error> {
		return Ok(balance);
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_: ()) {}
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = pallet_balances::Pallet<Runtime>;
	type RejectOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod; //Cannot be 0: Error: Thread 'tokio-runtime-worker' panicked at 'attempt to calculate the remainder with a divisor of zero
	type Burn = (); //No burn
	type BurnDestination = (); //No burn
	type SpendFunds = (); //No spend, no bounty
	type MaxApprovals = MaxApprovals;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>; //No spend, no bounty
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = NoConversion;
	type PayoutPeriod = PayoutSpendPeriod;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_encointer_reputation_commitments::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_encointer_reputation_commitments::WeightInfo<Runtime>;
}

impl pallet_encointer_faucet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type PalletId = FaucetPalletId;
	type WeightInfo = weights::pallet_encointer_faucet::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxProofSize: u32 = 256;
	pub const MaxVkSize: u32 = 2048;
}

impl pallet_encointer_offline_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_encointer_offline_payment::WeightInfo<Runtime>;
	type Currency = Balances;
	type MaxProofSize = MaxProofSize;
	type MaxVkSize = MaxVkSize;
	type TrustedSetupOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const MaxRingSize: u32 = 255;
	pub const RingChunkSize: u32 = 100;
}

impl pallet_encointer_reputation_rings::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_encointer_reputation_rings::WeightInfo<Runtime>;
	type MaxRingSize = MaxRingSize;
	type ChunkSize = RingChunkSize;
}

parameter_types! {
	pub const ConfirmationPeriod: Moment = 5 * 60 * 1000; // [ms]
	pub const ProposalLifetime: Moment = 20 * 60 * 1000; // [ms]
}

impl pallet_encointer_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxReputationCount = ConstU32<64>;
	type ConfirmationPeriod = ConfirmationPeriod;
	type ProposalLifetime = ProposalLifetime;
	type MinTurnout = ConstU128<1>; // permill of electorate: 1 = 0.1%, 50 = 5.0%
	type WeightInfo = weights::pallet_encointer_democracy::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TreasuriesPalletId: PalletId = PalletId(*b"trsrysId");
}
impl pallet_encointer_treasuries::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = pallet_balances::Pallet<Runtime>;
	type PalletId = TreasuriesPalletId;
	// Make our live easier by using the same type as in the parachain
	type AssetKind = VersionedLocatableAsset;
	type Paymaster = NoAssetPayments;
	type WeightInfo = weights::pallet_encointer_treasuries::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = MockAssetArguments;
}

/// Type that fails when we try to pay out a non-native asset as a result of a `SpendAsset` or
/// a swap of an `AssetOption`, as we only support this on the parachain.
pub struct NoAssetPayments;

impl pallet_encointer_treasuries::Transfer for NoAssetPayments {
	type Balance = Balance;
	type Payer = AccountId;
	type Beneficiary = AccountId;
	type AssetKind = VersionedLocatableAsset;
	type Id = ();
	type Error = alloc::string::String;

	fn transfer(
		_: &Self::Payer,
		_: &Self::Beneficiary,
		_: Self::AssetKind,
		_: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		Err("No asset payment allowed in this runtime config".into())
	}

	fn check_payment(_: Self::Id) -> PaymentStatus {
		PaymentStatus::Failure
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(
		_: &Self::Payer,
		_: &Self::Beneficiary,
		_: Self::AssetKind,
		_: Self::Balance,
	) {
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_concluded(_: Self::Id) {}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct MockAssetArguments;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_encointer_treasuries::benchmarking::ArgumentsFactory<VersionedLocatableAsset>
	for MockAssetArguments
{
	fn create_asset_kind(_: u32) -> VersionedLocatableAsset {
		// Just a dummy to make it compile
		use xcm::latest::{AssetId as A, Location as L};
		(L::here(), A(L::here())).into()
	}
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip::{Pallet, Storage} = 2,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 5,

		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Config<T>, Event<T>} = 11,
		AssetTxPayment: pallet_asset_tx_payment::{Pallet, Storage, Event<T>} = 12,


		Aura: pallet_aura::{Pallet, Config<T>} = 23,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config<T>, Event} = 25,

		Utility: pallet_utility::{Pallet, Call, Event} = 40,
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 44,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 48,
		Treasury: pallet_treasury::{Pallet, Call, Storage, Event<T>} = 49,

		EncointerScheduler: pallet_encointer_scheduler::{Pallet, Call, Storage, Config<T>, Event} = 60,
		EncointerCeremonies: pallet_encointer_ceremonies::{Pallet, Call, Storage, Config<T>, Event<T>} = 61,
		EncointerCommunities: pallet_encointer_communities::{Pallet, Call, Storage, Config<T>, Event<T>} = 62,
		EncointerBalances: pallet_encointer_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 63,
		EncointerBazaar: pallet_encointer_bazaar::{Pallet, Call, Storage, Event<T>} = 64,
		EncointerReputationCommitments: pallet_encointer_reputation_commitments::{Pallet, Call, Storage, Event<T>} = 65,
		EncointerFaucet: pallet_encointer_faucet::{Pallet, Call, Storage, Config<T>, Event<T>} = 66,
		EncointerDemocracy: pallet_encointer_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 67,
		EncointerTreasuries: pallet_encointer_treasuries::{Pallet, Call, Storage, Event<T>} = 68,
		EncointerOfflinePayment: pallet_encointer_offline_payment::{Pallet, Call, Storage, Event<T>} = 69,
		EncointerReputationRings: pallet_encointer_reputation_rings::{Pallet, Call, Storage, Event<T>} = 70,

	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type TxExtension = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_asset_tx_payment::ChargeAssetTxPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

/// storage migrations to be applied upon runtime upgrade
pub type Migrations = (pallet_encointer_democracy::migrations::v2::MigrateV1toV2<Runtime>);

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use super::*;

	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_encointer_balances, EncointerBalances]
		[pallet_encointer_bazaar, EncointerBazaar]
		[pallet_encointer_ceremonies, EncointerCeremonies]
		[pallet_encointer_communities, EncointerCommunities]
		[pallet_encointer_democracy, EncointerDemocracy]
		[pallet_encointer_faucet, EncointerFaucet]
		[pallet_encointer_offline_payment, EncointerOfflinePayment]
		[pallet_encointer_reputation_commitments, EncointerReputationCommitments]
		[pallet_encointer_reputation_rings, EncointerReputationRings]
		[pallet_encointer_scheduler, EncointerScheduler]
		[pallet_encointer_treasuries, EncointerTreasuries]
	);

	impl frame_system_benchmarking::Config for Runtime {
		fn setup_set_code_requirements(_code: &Vec<u8>) -> Result<(), BenchmarkError> {
			unimplemented!("fixme #397: runtime benchmarks are not really implemented in general");
		}

		fn verify_set_code() {}
	}

	pub use frame_benchmarking::{BenchmarkBatch, BenchmarkError, BenchmarkList, Benchmarking};
	pub use frame_support::traits::{StorageInfoTrait, TrackedStorageKey, WhitelistedStorageKeys};
	pub use frame_system_benchmarking::Pallet as SystemBench;
}

#[cfg(feature = "runtime-benchmarks")]
use benches::*;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
				Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as BlockT>::LazyBlock,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			pallet_aura::Authorities::<Runtime>::get().into_inner()
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_encointer_ceremonies_rpc_runtime_api::CeremoniesApi<Block, AccountId, Moment> for Runtime {
		fn get_reputations(account: &AccountId) -> Vec<(CeremonyIndexType, CommunityReputation)> {
			EncointerCeremonies::get_reputations(account)
		}
		fn get_aggregated_account_data(cid:CommunityIdentifier, account: &AccountId) -> AggregatedAccountData<AccountId, Moment> {
			EncointerCeremonies::get_aggregated_account_data(cid, account)
		}
		fn get_ceremony_info() -> CeremonyInfo {
			EncointerCeremonies::get_ceremony_info()
		}
	}

	impl pallet_encointer_communities_rpc_runtime_api::CommunitiesApi<Block, AccountId, BlockNumber> for Runtime {
		fn get_all_balances(account: &AccountId) -> Vec<(CommunityIdentifier, BalanceEntry<BlockNumber>)> {
			EncointerCommunities::get_all_balances(account)
		}

		fn get_cids() -> Vec<CommunityIdentifier> {
			EncointerCommunities::get_cids()
		}

		fn get_name(cid: &CommunityIdentifier) -> Option<PalletString> {
			EncointerCommunities::get_name(cid)
		}

		fn get_locations(cid: &CommunityIdentifier) -> Vec<Location> {
			EncointerCommunities::get_locations(cid)
		}

	}

	impl pallet_encointer_bazaar_rpc_runtime_api::BazaarApi<Block, AccountId> for Runtime {
		fn get_offerings(business: &BusinessIdentifier<AccountId>) -> Vec<OfferingData>{
			EncointerBazaar::get_offerings(business)
		}

		fn get_businesses(community: &CommunityIdentifier) -> Vec<(AccountId, BusinessData)>{
			EncointerBazaar::get_businesses(community)
		}
	}

	impl encointer_balances_tx_payment_rpc_runtime_api::BalancesTxPaymentApi<Block, Balance, AssetId, AssetBalance> for Runtime {
		fn balance_to_asset_balance(amount: Balance, asset_id: AssetId) -> Result<AssetBalance, encointer_balances_tx_payment_rpc_runtime_api::Error> {
			BalanceToCommunityBalance::<Runtime>::to_asset_balance(amount, asset_id).map_err(|_e|
				encointer_balances_tx_payment_rpc_runtime_api::Error::RuntimeError
			)
		}
	}

	impl pallet_encointer_treasuries_rpc_runtime_api::TreasuriesApi<Block, AccountId> for Runtime {

		fn get_community_treasury_account_unchecked(maybecid: &Option<CommunityIdentifier>) -> AccountId {
			EncointerTreasuries::get_community_treasury_account_unchecked(*maybecid)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (Vec<BenchmarkList>,Vec<StorageInfo>) {
			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);
			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(config: frame_benchmarking::BenchmarkConfig) -> Result<Vec<BenchmarkBatch>, alloc::string::String> {
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();
			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);
			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: <Block as BlockT>::LazyBlock,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}
}
