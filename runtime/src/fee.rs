// Copyright 2018 Chainpool

//use balances::Call as BalancesCall;
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
            _ => None,
        }
    }
}
