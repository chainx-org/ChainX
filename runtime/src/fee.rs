// Copyright 2018 Chainpool

//use balances::Call as BalancesCall;
use xassets::Call as XAssetsCall;
use xbitcoin::Call as XbitcoinCall;
use xprocess::Call as XAssetsProcessCall;
use xstaking::Call as XStakingCall;

use Call;

pub trait CheckFee {
    fn check_fee(&self) -> Option<u64>;
}

impl CheckFee for Call {
    fn check_fee(&self) -> Option<u64> {
        // ret fee_power,     total_fee = base_fee * fee_power + byte_fee * bytes
        match self {
            // xassets
            Call::XAssets(call) => match call {
                XAssetsCall::transfer(_, _, _, _) => Some(10),
                // root
                XAssetsCall::set_balance(_, _, _) => Some(0),
                XAssetsCall::register_asset(_, _, _) => Some(0),
                XAssetsCall::cancel_asset(_) => Some(0),
                _ => None,
            },
            Call::XAssetsProcess(call) => match call {
                XAssetsProcessCall::withdraw(_, _, _, _) => Some(3),
                _ => None,
            },
            // xbridge
            Call::XBridgeOfBTC(call) => match call {
                XbitcoinCall::push_header(_) => Some(20),
                XbitcoinCall::push_transaction(_) => Some(10),
                _ => None,
            },
            Call::XStaking(call) => match call {
                XStakingCall::register(_, _, _, _, _, _) => Some(100),
                XStakingCall::refresh(_, _) => Some(100),
                XStakingCall::nominate(_, _, _) => Some(5),
                XStakingCall::unnominate(_, _, _) => Some(3),
                XStakingCall::unfreeze(_, _) => Some(2),
                XStakingCall::claim(_) => Some(3),
                _ => None,
            },
            _ => None,
        }
    }
}
