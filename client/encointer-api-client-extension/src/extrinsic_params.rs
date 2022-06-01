use codec::{Decode, Encode};
use encointer_primitives::communities::CommunityIdentifier;
use substrate_api_client::{
	BaseExtrinsicParams, BaseExtrinsicParamsBuilder, SubstrateDefaultSignedExtra,
	UncheckedExtrinsicV4,
};

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction and pay in asset fees
pub type CommunityCurrencyTipExtrinsicParams = BaseExtrinsicParams<AssetTip>;
/// A builder which leads to [`CommunityCurrencyTipExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type CommunityCurrencyTipExtrinsicParamsBuilder = BaseExtrinsicParamsBuilder<AssetTip>;

pub type EncointerXt<Call> = UncheckedExtrinsicV4<Call, SubstrateDefaultSignedExtra<AssetTip>>;

/// A tip payment made in the form of a specific asset.
#[derive(Copy, Clone, Debug, Default, Decode, Encode, Eq, PartialEq)]
pub struct AssetTip {
	#[codec(compact)]
	tip: u128,
	asset: Option<CommunityIdentifier>,
}

impl AssetTip {
	/// Create a new tip of the amount provided.
	pub fn new(amount: u128) -> Self {
		AssetTip { tip: amount, asset: None }
	}

	/// Designate the tip as being of a particular asset class.
	/// If this is not set, then the native currency is used.
	pub fn of_asset(mut self, asset: CommunityIdentifier) -> Self {
		self.asset = Some(asset);
		self
	}
}

impl From<u128> for AssetTip {
	fn from(n: u128) -> Self {
		AssetTip::new(n)
	}
}

impl From<AssetTip> for u128 {
	fn from(tip: AssetTip) -> Self {
		tip.tip
	}
}
