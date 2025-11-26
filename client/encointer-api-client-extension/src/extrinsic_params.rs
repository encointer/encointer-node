use crate::{Api, CommunitiesApi, ExtrinsicAddress};
use encointer_node_notee_runtime::{Hash, Nonce, Signature};
use encointer_primitives::communities::CommunityIdentifier;
use parity_scale_codec::{Decode, Encode};
use substrate_api_client::ac_primitives::{
	GenericAdditionalParams, GenericExtrinsicParams, GenericTxExtension, UncheckedExtrinsic,
};

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction and pay in asset fees
pub type CommunityCurrencyTipExtrinsicParams<T> = GenericExtrinsicParams<T, CommunityCurrencyTip>;
/// A builder which leads to [`CommunityCurrencyTipExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type CommunityCurrencyTipExtrinsicParamsBuilder =
	GenericAdditionalParams<CommunityCurrencyTip, Hash>;

pub type EncointerXt<Call> = UncheckedExtrinsic<
	ExtrinsicAddress,
	Call,
	Signature,
	GenericTxExtension<CommunityCurrencyTip, Nonce>,
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

pub async fn set_api_extrisic_params_builder(api: &mut Api, tx_payment_cid_arg: Option<&str>) {
	let mut tx_params = CommunityCurrencyTipExtrinsicParamsBuilder::new().tip(0);
	if let Some(tx_payment_cid) = tx_payment_cid_arg {
		tx_params = tx_params.tip(
			CommunityCurrencyTip::new(0).of_community(api.verify_cid(tx_payment_cid, None).await),
		);
	}
	let _ = &api.set_additional_params(tx_params);
}
