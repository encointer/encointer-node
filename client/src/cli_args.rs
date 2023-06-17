use clap::{App, Arg, ArgMatches};
use sp_core::{bytes, H256 as Hash};

const ACCOUNT_ARG: &str = "accountid";
const FAUCET_ACCOUNT_ARG: &str = "faucet-account";
const FAUCET_BENEFICIARY_ARG: &str = "faucet-beneficiary";
const SEED_ARG: &str = "seed";
const SIGNER_ARG: &str = "signer";
const CID_ARG: &str = "cid";
const ATTESTEES_ARG: &str = "attestees";
const WHITELIST_ARG: &str = "whitelist";
const CEREMONY_INDEX_ARG: &str = "ceremony-index";
const IPFS_CID_ARG: &str = "ipfs-cid";
const FAUCET_NAME_ARG: &str = "faucet-name";
const BOOTSTRAPPER_ARG: &str = "bootstrapper";
const FUNDEES_ARG: &str = "fundees";
const FROM_CINDEX_ARG: &str = "from-cindex";
const TO_CINDEX_ARG: &str = "to-cindex";
const CINDEX_ARG: &str = "cindex";
const ENDORSEES_ARG: &str = "endorsees";
const TIME_OFFSET_ARG: &str = "time-offset";
const ALL_FLAG: &str = "all";
const DRYRUN_FLAG: &str = "dryrun";
const TX_PAYMENT_CID_ARG: &str = "tx-payment-cid";
const MEETUP_INDEX_ARG: &str = "meetup-index";
const AT_BLOCK_ARG: &str = "at";
const VERBOSE_FLAG: &str = "verbose";
const FAUCET_BALANCE_ARG: &str = "faucet-balance";
const FAUCET_DRIP_AMOUNT_ARG: &str = "faucet-drip-amount";
const FAUCET_RESERVE_AMOUNT_ARG: &str = "faucet-reserve-amount";

pub trait EncointerArgs<'b> {
	fn account_arg(self) -> Self;
	fn faucet_account_arg(self) -> Self;
	fn faucet_beneficiary_arg(self) -> Self;
	fn seed_arg(self) -> Self;
	fn signer_arg(self, help: &'b str) -> Self;
	fn optional_cid_arg(self) -> Self;
	fn attestees_arg(self) -> Self;
	fn whitelist_arg(self) -> Self;
	fn ceremony_index_arg(self) -> Self;
	fn ipfs_cid_arg(self) -> Self;
	fn faucet_name_arg(self) -> Self;
	fn bootstrapper_arg(self) -> Self;
	fn fundees_arg(self) -> Self;
	#[allow(clippy::wrong_self_convention)]
	fn from_cindex_arg(self) -> Self;
	fn to_cindex_arg(self) -> Self;
	fn cindex_arg(self) -> Self;
	fn endorsees_arg(self) -> Self;
	fn time_offset_arg(self) -> Self;
	fn all_flag(self) -> Self;
	fn dryrun_flag(self) -> Self;
	fn tx_payment_cid_arg(self) -> Self;
	fn meetup_index_arg(self) -> Self;
	fn at_block_arg(self) -> Self;
	fn verbose_flag(self) -> Self;
	fn faucet_balance_arg(self) -> Self;
	fn faucet_drip_amount_arg(self) -> Self;
	fn faucet_reserve_amount_arg(self) -> Self;
}

pub trait EncointerArgsExtractor {
	fn account_arg(&self) -> Option<&str>;
	fn faucet_account_arg(&self) -> Option<&str>;
	fn faucet_beneficiary_arg(&self) -> Option<&str>;
	fn seed_arg(&self) -> Option<&str>;
	fn signer_arg(&self) -> Option<&str>;
	fn cid_arg(&self) -> Option<&str>;
	fn attestees_arg(&self) -> Option<Vec<&str>>;
	fn whitelist_arg(&self) -> Option<Vec<&str>>;
	fn ceremony_index_arg(&self) -> Option<i32>;
	fn ipfs_cid_arg(&self) -> Option<&str>;
	fn faucet_name_arg(&self) -> Option<&str>;
	fn bootstrapper_arg(&self) -> Option<&str>;
	fn fundees_arg(&self) -> Option<Vec<&str>>;
	#[allow(clippy::wrong_self_convention)]
	fn from_cindex_arg(&self) -> Option<i32>;
	fn to_cindex_arg(&self) -> Option<i32>;
	fn cindex_arg(&self) -> Option<i32>;
	fn endorsees_arg(&self) -> Option<Vec<&str>>;
	fn time_offset_arg(&self) -> Option<i32>;
	fn all_flag(&self) -> bool;
	fn dryrun_flag(&self) -> bool;
	fn tx_payment_cid_arg(&self) -> Option<&str>;
	fn meetup_index_arg(&self) -> Option<u64>;
	fn at_block_arg(&self) -> Option<Hash>;
	fn verbose_flag(&self) -> bool;
	fn faucet_balance_arg(&self) -> Option<u128>;
	fn faucet_drip_amount_arg(&self) -> Option<u128>;
	fn faucet_reserve_amount_arg(&self) -> Option<u128>;
}

impl<'a, 'b> EncointerArgs<'b> for App<'a, 'b> {
	fn account_arg(self) -> Self {
		self.arg(
			Arg::with_name(ACCOUNT_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help("AccountId in ss58check format"),
		)
	}

	fn faucet_account_arg(self) -> Self {
		self.arg(
			Arg::with_name(FAUCET_ACCOUNT_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help("faucet account in ss58check format"),
		)
	}

	fn faucet_beneficiary_arg(self) -> Self {
		self.arg(
			Arg::with_name(FAUCET_BENEFICIARY_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help("faucet account in ss58check format"),
		)
	}

	fn seed_arg(self) -> Self {
		self.arg(
			Arg::with_name(SEED_ARG)
				.takes_value(true)
				.required(false)
				.value_name("SS58")
				.help("Seed, mnemonic of suri"),
		)
	}

	fn signer_arg(self, help: &'b str) -> Self {
		self.arg(
			Arg::with_name(SIGNER_ARG)
				.short("s")
				.long("signer")
				.takes_value(true)
				.required(false)
				.value_name("suri, seed , mnemonic or SS58 in keystore")
				.help(help)
				.conflicts_with(MEETUP_INDEX_ARG),
		)
	}

	fn optional_cid_arg(self) -> Self {
		self.arg(
			Arg::with_name(CID_ARG)
				.short("c")
				.long("cid")
				.global(true)
				.takes_value(true)
				.value_name("STRING")
				.help("community identifier, base58 encoded"),
		)
	}

	fn attestees_arg(self) -> Self {
		self.arg(
			Arg::with_name(ATTESTEES_ARG)
				.takes_value(true)
				.required(true)
				.multiple(true)
				.min_values(2),
		)
	}

	fn whitelist_arg(self) -> Self {
		self.arg(
			Arg::with_name(WHITELIST_ARG)
				.takes_value(true)
				.required(true)
				.multiple(true)
				.min_values(1),
		)
	}

	fn ceremony_index_arg(self) -> Self {
		self.arg(
			Arg::with_name(CEREMONY_INDEX_ARG)
				.takes_value(true)
				.allow_hyphen_values(true)
				.help(
					"If positive, absolute index. If negative, current_index -i. 0 is not allowed",
				),
		)
	}

	fn ipfs_cid_arg(self) -> Self {
		self.arg(
			Arg::with_name(IPFS_CID_ARG)
				.long("ipfs-cid")
				.required(true)
				.takes_value(true)
				.value_name("STRING")
				.help("ipfs content identifier, base58 encoded"),
		)
	}

	fn faucet_name_arg(self) -> Self {
		self.arg(
			Arg::with_name(FAUCET_NAME_ARG)
				.required(true)
				.takes_value(true)
				.value_name("STRING")
				.help("faucet name"),
		)
	}

	fn bootstrapper_arg(self) -> Self {
		self.arg(
			Arg::with_name(BOOTSTRAPPER_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help("Bootstrapper in ss58check format"),
		)
	}

	fn fundees_arg(self) -> Self {
		self.arg(
			Arg::with_name(FUNDEES_ARG)
				.takes_value(true)
				.required(true)
				.value_name("FUNDEE")
				.multiple(true)
				.min_values(1)
				.help("Account(s) to be funded, ss58check encoded"),
		)
	}
	fn from_cindex_arg(self) -> Self {
		self.arg(
			Arg::with_name(FROM_CINDEX_ARG)
				.takes_value(true)
				.required(true)
				.value_name("FROM")
				.help("first ceremony index to be purged"),
		)
	}
	fn to_cindex_arg(self) -> Self {
		self.arg(
			Arg::with_name(TO_CINDEX_ARG)
				.takes_value(true)
				.required(true)
				.value_name("TO")
				.help("last ceremony index to be purged"),
		)
	}
	fn cindex_arg(self) -> Self {
		self.arg(
			Arg::with_name(CINDEX_ARG)
				.takes_value(true)
				.required(true)
				.value_name("CINDEX")
				.help("cindex"),
		)
	}
	fn endorsees_arg(self) -> Self {
		self.arg(
			Arg::with_name(ENDORSEES_ARG)
				.short("-e")
				.long("-endorsees")
				.takes_value(true)
				.required(true)
				.value_name("ENDORSEE")
				.multiple(true)
				.min_values(1)
				.help("Account(s) to be endorsed, ss58check encoded"),
		)
	}
	fn time_offset_arg(self) -> Self {
		self.arg(
			Arg::with_name(TIME_OFFSET_ARG)
				.takes_value(true)
				.required(true)
				.value_name("TIME_OFFSET")
				.help("signed value in milliseconds"),
		)
	}
	fn all_flag(self) -> Self {
		self.arg(
			Arg::with_name(ALL_FLAG)
				.short("a")
				.long("all")
				.takes_value(false)
				.required(false)
				.help("list all community currency balances for account"),
		)
	}
	fn dryrun_flag(self) -> Self {
		self.arg(
			Arg::with_name(DRYRUN_FLAG)
				.short("d")
				.long("dryrun")
				.takes_value(false)
				.required(false)
				.help("print the encoded call instead of signing and sending an extrinsic"),
		)
	}
	fn tx_payment_cid_arg(self) -> Self {
		self.arg(
			Arg::with_name(TX_PAYMENT_CID_ARG)
				.long("tx-payment-cid")
				.global(true)
				.takes_value(true)
				.required(false)
				.value_name("STRING")
				.help("cid of the community currency in which tx fees should be paid"),
		)
	}
	fn meetup_index_arg(self) -> Self {
		self.arg(
			Arg::with_name(MEETUP_INDEX_ARG)
				.long("meetup-index")
				.takes_value(true)
				.required(false)
				.value_name("MEETUP_INDEX")
				.conflicts_with(ALL_FLAG)
				.help("the meetup index for which to claim rewards"),
		)
	}
	fn at_block_arg(self) -> Self {
		self.arg(
			Arg::with_name(AT_BLOCK_ARG)
				.long("at")
				.global(true)
				.takes_value(true)
				.required(false)
				.value_name("STRING")
				.help("block hash at which to query"),
		)
	}
	fn verbose_flag(self) -> Self {
		self.arg(
			Arg::with_name(VERBOSE_FLAG)
				.short("v")
				.long("verbose")
				.global(true)
				.takes_value(false)
				.required(false)
				.help("print extra information"),
		)
	}
	fn faucet_balance_arg(self) -> Self {
		self.arg(
			Arg::with_name(FAUCET_BALANCE_ARG)
				.takes_value(true)
				.required(true)
				.value_name("FAUCET_BALANCE")
				.help("faucet balance"),
		)
	}
	fn faucet_drip_amount_arg(self) -> Self {
		self.arg(
			Arg::with_name(FAUCET_DRIP_AMOUNT_ARG)
				.takes_value(true)
				.required(true)
				.value_name("FAUCET_DRIP_AMOUNT")
				.help("faucet drip amount"),
		)
	}
	fn faucet_reserve_amount_arg(self) -> Self {
		self.arg(
			Arg::with_name(FAUCET_RESERVE_AMOUNT_ARG)
				.takes_value(true)
				.required(true)
				.value_name("FAUCET_RESERVE_AMOUNT")
				.help("faucet reserve amount"),
		)
	}
}

impl<'a> EncointerArgsExtractor for ArgMatches<'a> {
	fn account_arg(&self) -> Option<&str> {
		self.value_of(ACCOUNT_ARG)
	}

	fn faucet_account_arg(&self) -> Option<&str> {
		self.value_of(FAUCET_ACCOUNT_ARG)
	}

	fn faucet_beneficiary_arg(&self) -> Option<&str> {
		self.value_of(FAUCET_BENEFICIARY_ARG)
	}

	fn seed_arg(&self) -> Option<&str> {
		self.value_of(SEED_ARG)
	}

	fn signer_arg(&self) -> Option<&str> {
		self.value_of(SIGNER_ARG)
	}

	fn cid_arg(&self) -> Option<&str> {
		self.value_of(CID_ARG)
	}

	fn attestees_arg(&self) -> Option<Vec<&str>> {
		self.values_of(ATTESTEES_ARG).map(|c| c.collect())
	}

	fn whitelist_arg(&self) -> Option<Vec<&str>> {
		self.values_of(WHITELIST_ARG).map(|c| c.collect())
	}

	fn ceremony_index_arg(&self) -> Option<i32> {
		self.value_of(CEREMONY_INDEX_ARG).map(|v| v.parse().unwrap())
	}

	fn ipfs_cid_arg(&self) -> Option<&str> {
		self.value_of(IPFS_CID_ARG)
	}

	fn faucet_name_arg(&self) -> Option<&str> {
		self.value_of(FAUCET_NAME_ARG)
	}

	fn bootstrapper_arg(&self) -> Option<&str> {
		self.value_of(BOOTSTRAPPER_ARG)
	}

	fn fundees_arg(&self) -> Option<Vec<&str>> {
		self.values_of(FUNDEES_ARG).map(|v| v.collect())
	}

	fn from_cindex_arg(&self) -> Option<i32> {
		self.value_of(FROM_CINDEX_ARG).map(|v| v.parse().unwrap())
	}
	fn to_cindex_arg(&self) -> Option<i32> {
		self.value_of(TO_CINDEX_ARG).map(|v| v.parse().unwrap())
	}
	fn cindex_arg(&self) -> Option<i32> {
		self.value_of(CINDEX_ARG).map(|v| v.parse().unwrap())
	}
	fn endorsees_arg(&self) -> Option<Vec<&str>> {
		self.values_of(ENDORSEES_ARG).map(|v| v.collect())
	}

	fn time_offset_arg(&self) -> Option<i32> {
		self.value_of(TIME_OFFSET_ARG).map(|v| v.parse().unwrap())
	}
	fn all_flag(&self) -> bool {
		self.is_present(ALL_FLAG)
	}
	fn dryrun_flag(&self) -> bool {
		self.is_present(DRYRUN_FLAG)
	}
	fn tx_payment_cid_arg(&self) -> Option<&str> {
		self.value_of(TX_PAYMENT_CID_ARG)
	}
	fn meetup_index_arg(&self) -> Option<u64> {
		self.value_of(MEETUP_INDEX_ARG).map(|v| v.parse().unwrap())
	}
	fn at_block_arg(&self) -> Option<Hash> {
		self.value_of(AT_BLOCK_ARG).map(|hex| {
			let vec = bytes::from_hex(hex)
				.unwrap_or_else(|_| panic!("bytes::from_hex failed, data is: {hex}"));
			if vec.len() != 32 {
				panic!("in at_block_arg fn, vec is: {vec:#?}");
			}
			Hash::from_slice(&vec)
		})
	}
	fn verbose_flag(&self) -> bool {
		self.is_present(VERBOSE_FLAG)
	}
	fn faucet_balance_arg(&self) -> Option<u128> {
		self.value_of(FAUCET_BALANCE_ARG).map(|v| v.parse().unwrap())
	}
	fn faucet_drip_amount_arg(&self) -> Option<u128> {
		self.value_of(FAUCET_DRIP_AMOUNT_ARG).map(|v| v.parse().unwrap())
	}
	fn faucet_reserve_amount_arg(&self) -> Option<u128> {
		self.value_of(FAUCET_RESERVE_AMOUNT_ARG).map(|v| v.parse().unwrap())
	}
}
