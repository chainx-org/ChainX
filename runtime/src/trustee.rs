// Copyright 2018-2019 Chainpool.

use support::dispatch::{Dispatchable, Result};
use system;

use super::{AccountId, Call};
use xbitcoin::Call as XBitcoinCall;
use xbridge_features::Call as XBridgeFeaturesCall;
use xmultisig::LimitedCall;
use xsupport::{error, info};

pub struct TrusteeCall(Call);

impl From<Call> for TrusteeCall {
    fn from(call: Call) -> Self {
        TrusteeCall(call)
    }
}

impl LimitedCall<AccountId> for TrusteeCall {
    fn allow(&self) -> bool {
        // only allow trustee function
        match &self.0 {
            Call::XBridgeOfBTC(call) => match call {
                XBitcoinCall::set_btc_withdrawal_fee_by_trustees(..) => true,
                XBitcoinCall::set_btc_deposit_limit_by_trustees(..) => true,
                XBitcoinCall::fix_withdrawal_state_by_trustees(..) => true,
                XBitcoinCall::remove_pending_by_trustees(..) => true,
                _ => false,
            },
            Call::XBridgeFeatures(call) => match call {
                XBridgeFeaturesCall::transition_trustee_session(..) => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn exec(&self, exerciser: &AccountId) -> Result {
        if !self.allow() {
            error!(
                "[LimitedCall]|not allow to exec this call for trustee role now|exerciser:{:?}",
                exerciser
            );
            return Err("not allow to exec this call for trustee role now");
        }
        info!(
            "trustee exec|try to exec from multisig addr:{:?}",
            exerciser
        );
        let origin = system::RawOrigin::Signed(exerciser.clone()).into();
        if let Err(e) = self.0.clone().dispatch(origin) {
            if e == "bad origin: expected to be a root origin" {
                info!("failed by executing from addr, try to use root to exec it");
                let origin = system::RawOrigin::Root.into();
                return self.0.clone().dispatch(origin);
            }
            return Err(e);
        }
        Ok(())
    }
}
