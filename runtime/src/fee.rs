// Copyright 2018 Chainpool

//use balances::Call as BalancesCall;
use bitcoin::Call as BitcoinCall;
use sdot::Call as SdotCall;
use sudo::Call as SudoCall;
use xassets::Call as XAssetsCall;
use xprocess::Call as XAssetsProcessCall;
use xspot::Call as XSpotCall;
use xstaking::Call as XStakingCall;
use xtokens::Call as XTokensCall;

use Call;

pub trait CheckFee {
    fn check_fee(&self) -> Option<u64>;
}

impl CheckFee for Call {
    /// Return fee_power, which is part of the total_fee.
    /// total_fee = base_fee * fee_power + byte_fee * bytes
    ///
    /// fee_power = power_per_call
    fn check_fee(&self) -> Option<u64> {
        let base_power = match self {
            // xassets
            Call::XAssets(call) => match call {
                XAssetsCall::transfer(_, _, _, _) => Some(10),
                // root
                XAssetsCall::set_balance(_, _, _) => Some(0),
                XAssetsCall::register_asset(_, _, _) => Some(0),
                XAssetsCall::revoke_asset(_) => Some(0),
                _ => None,
            },
            Call::XAssetsProcess(call) => match call {
                XAssetsProcessCall::withdraw(_, _, _, _) => Some(3),
                _ => None,
            },
            // xbridge
            Call::XBridgeOfBTC(call) => match call {
                BitcoinCall::push_header(_) => Some(20),
                BitcoinCall::push_transaction(_) => Some(10),
                BitcoinCall::create_withdraw_tx(_, _) => Some(50),
                BitcoinCall::sign_withdraw_tx(_, _) => Some(50),
                _ => None,
            },
            // xmining
            Call::XStaking(call) => match call {
                XStakingCall::register(_) => Some(100),
                XStakingCall::refresh(_, _, _, _) => Some(5),
                XStakingCall::nominate(_, _, _) => Some(5),
                XStakingCall::unnominate(_, _, _) => Some(3),
                XStakingCall::unfreeze(_, _) => Some(2),
                XStakingCall::claim(_) => Some(3),
                XStakingCall::setup_trustee(_, _, _, _) => Some(5),
                _ => None,
            },
            Call::XTokens(call) => match call {
                XTokensCall::claim(_) => Some(3),
                _ => None,
            },
            Call::XSpot(call) => match call {
                XSpotCall::put_order(_, _, _, _, _) => Some(8),
                XSpotCall::cancel_order(_, _) => Some(2),
                _ => None,
            },
            Call::Sudo(call) => match call {
                SudoCall::sudo(_) => Some(1),
                SudoCall::set_key(_) => Some(1),
                _ => None,
            },
            Call::XBridgeOfSDOT(call) => match call {
                SdotCall::claim(_, _, _) => Some(1),
                _ => None,
            },
            _ => None,
        };
        base_power
    }
}
