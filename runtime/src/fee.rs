// Copyright 2018 Chainpool

//use balances::Call as BalancesCall;
use xbitcoin::Call as XbitcoinCall;
use xassets::Call as XAssetsCall;
use xprocess::Call as XAssetsProcessCall;

use Call;

pub trait CheckFee {
    fn check_fee(&self) -> Option<u64>;
}

impl CheckFee for Call {
    fn check_fee(&self) -> Option<u64> {
        // ret fee_power,     total_fee = base_fee * fee_power + byte_fee * bytes
        match self {
            //            Call::Balances(call) => match call {
            //                BalancesCall::transfer(_, _) => Some(1),
            //                _ => None,
            //            },
            Call::XBridgeOfBTC(call) => match call {
                XbitcoinCall::push_header(_) => Some(20),
                XbitcoinCall::push_transaction(_) => Some(10),
                _ => None,
            },
            Call::XAssets(call) => match call {
                XAssetsCall::transfer(_,_,_,_) => Some(1),
                _=>None,
            },
            Call::XAssetsProcess(call) => match call {
                XAssetsProcessCall::withdraw(_,_,_,_) => Some(3),
                _=>None,
            },
            _ => None,
        }
    }
}
