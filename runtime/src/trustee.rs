// Copyright 2018-2019 Chainpool.

use support::dispatch::{Dispatchable, Result};
use system;

use super::{AccountId, Call};
use xbitcoin::Call as XBitcoinCall;
use xmultisig::Call as XMultiSigCall;
use xmultisig::TrusteeCall;
use xsupport::{error, info};

impl TrusteeCall<AccountId> for Call {
    fn allow(&self) -> bool {
        // only allow trustee function
        match self {
            Call::XBridgeOfBTC(call) => match call {
                XBitcoinCall::set_btc_withdrawal_fee(_) => true,
                _ => false,
            },
            Call::XMultiSig(call) => match call {
                XMultiSigCall::transition_trustee_session(_, _) => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn exec(&self, exerciser: &AccountId) -> Result {
        if !self.allow() {
            error!("[TrusteeCall]|");
            return Err("not allow to exec this call for trustee role now");
        }
        info!(
            "trustee exec|try to exec from multisig addr:{:?}",
            exerciser
        );
        let origin = system::RawOrigin::Signed(exerciser.clone()).into();
        if let Err(e) = self.clone().dispatch(origin) {
            if e == "bad origin: expected to be a root origin" {
                info!("failed by executing from addr, try to use root to exec it");
                let origin = system::RawOrigin::Root.into();
                return self.clone().dispatch(origin);
            }
            return Err(e);
        }
        Ok(())
    }
}
