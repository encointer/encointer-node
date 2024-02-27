use crate::ExtrinsicAddress;
use encointer_node_notee_runtime::{Hash, Index, Signature};
use encointer_primitives::communities::CommunityIdentifier;
use parity_scale_codec::{Decode, Encode};
use substrate_api_client::ac_primitives::{
	GenericAdditionalParams, GenericExtrinsicParams, GenericSignedExtra, UncheckedExtrinsicV4,
};

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction and pay in asset fees
pub type CommunityCurrencyTipExtrinsicParams<T> = GenericExtrinsicParams<T, CommunityCurrencyTip>;
/// A builder which leads to [`CommunityCurrencyTipExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type CommunityCurrencyTipExtrinsicParamsBuilder =
	GenericAdditionalParams<CommunityCurrencyTip, Hash>;

pub type EncointerXt<Call> = UncheckedExtrinsicV4<
	ExtrinsicAddress,
	Call,
	Signature,
	GenericSignedExtra<CommunityCurrencyTip, Index>,
>;

/// A tip payment made in the form of a specific asset.
#[derive(Copy, Clone, Debug, Default, Decode, Encode, Eq, PartialEq)]
pub struct CommunityCurrencyTip {
	#[codec(compact)]
	tip: u128,
	asset: Option<CommunityIdentifier>,
}

impl CommunityCurrencyTip {
	/// Create a new tip of the amount provided.
	pub fn new(amount: u128) -> Self {
		CommunityCurrencyTip { tip: amount, asset: None }
	}

	/// Designate the tip as being of a particular asset class.
	/// If this is not set, then the native currency is used.
	pub fn of_community(mut self, asset: CommunityIdentifier) -> Self {
		self.asset = Some(asset);
		self
	}
}

impl From<u128> for CommunityCurrencyTip {
	fn from(n: u128) -> Self {
		CommunityCurrencyTip::new(n)
	}
}

impl From<CommunityCurrencyTip> for u128 {
	fn from(tip: CommunityCurrencyTip) -> Self {
		tip.tip
	}
}
