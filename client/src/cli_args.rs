use clap::{App, Arg, ArgMatches};
use sp_core::H256 as Hash;

const ACCOUNT_ARG: &str = "accountid";
const SEED_ARG: &str = "seed";
const SIGNER_ARG: &str = "signer";
const CID_ARG: &str = "cid";
const ATTESTEES_ARG: &str = "attestees";
const CEREMONY_INDEX_ARG: &str = "ceremony-index";
const IPFS_CID_ARG: &str = "ipfs-cid";
const BOOTSTRAPPER_ARG: &str = "bootstrapper";
const FUNDEES_ARG: &str = "fundees";
const FROM_CINDEX_ARG: &str = "from-cindex";
const TO_CINDEX_ARG: &str = "to-cindex";
const ENDORSEES_ARG: &str = "endorsees";
const TIME_OFFSET_ARG: &str = "time-offset";
const ALL_FLAG: &str = "all";
const DRYRUN_FLAG: &str = "dryrun";
const TX_PAYMENT_CID_ARG: &str = "tx-payment-cid";
const MEETUP_INDEX_ARG: &str = "meetup-index";
const AT_BLOCK_ARG: &str = "at";

pub trait EncointerArgs<'b> {
	fn account_arg(self) -> Self;
	fn seed_arg(self) -> Self;
	fn signer_arg(self, help: &'b str) -> Self;
	fn optional_cid_arg(self) -> Self;
	fn attestees_arg(self) -> Self;
	fn ceremony_index_arg(self) -> Self;
	fn ipfs_cid_arg(self) -> Self;
	fn bootstrapper_arg(self) -> Self;
	fn fundees_arg(self) -> Self;
	#[allow(clippy::wrong_self_convention)]
	fn from_cindex_arg(self) -> Self;
	fn to_cindex_arg(self) -> Self;
	fn endorsees_arg(self) -> Self;
	fn time_offset_arg(self) -> Self;
	fn all_flag(self) -> Self;
	fn dryrun_flag(self) -> Self;
	fn tx_payment_cid_arg(self) -> Self;
	fn meetup_index_arg(self) -> Self;
	fn at_block_arg(self) -> Self;
}

pub trait EncointerArgsExtractor {
	fn account_arg(&self) -> Option<&str>;
	fn seed_arg(&self) -> Option<&str>;
	fn signer_arg(&self) -> Option<&str>;
	fn cid_arg(&self) -> Option<&str>;
	fn attestees_arg(&self) -> Option<Vec<&str>>;
	fn ceremony_index_arg(&self) -> Option<i32>;
	fn ipfs_cid_arg(&self) -> Option<&str>;
	fn bootstrapper_arg(&self) -> Option<&str>;
	fn fundees_arg(&self) -> Option<Vec<&str>>;
	#[allow(clippy::wrong_self_convention)]
	fn from_cindex_arg(&self) -> Option<i32>;
	fn to_cindex_arg(&self) -> Option<i32>;
	fn endorsees_arg(&self) -> Option<Vec<&str>>;
	fn time_offset_arg(&self) -> Option<i32>;
	fn all_flag(&self) -> bool;
	fn dryrun_flag(&self) -> bool;
	fn tx_payment_cid_arg(&self) -> Option<&str>;
	fn meetup_index_arg(&self) -> Option<u64>;
	fn at_block_arg(&self) -> Option<Hash>;
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
}

impl<'a> EncointerArgsExtractor for ArgMatches<'a> {
	fn account_arg(&self) -> Option<&str> {
		self.value_of(ACCOUNT_ARG)
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

	fn ceremony_index_arg(&self) -> Option<i32> {
		self.value_of(CEREMONY_INDEX_ARG).map(|v| v.parse().unwrap())
	}

	fn ipfs_cid_arg(&self) -> Option<&str> {
		self.value_of(IPFS_CID_ARG)
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
			let hexstr = hex.to_string();
			let hexstr = hexstr.trim_matches('\"').trim_start_matches("0x");
			let vec =
				hex::decode(hexstr).expect(&format!("hex::decode failed, data is: {:#?}", hexstr));
			if vec.len() != 32 {
				panic!("in at_block_arg fn, vec is: {:#?}", vec);
			}
			Hash::from_slice(&vec)
		})
	}
}
