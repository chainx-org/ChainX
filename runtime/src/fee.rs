// Copyright 2018 Chainpool

//use balances::Call as BalancesCall;
use bitcoin::Call as BitcoinCall;
use fee_manager::SwitchStore;
use sdot::Call as SdotCall;
use sudo::Call as SudoCall;
use xassets::Call as XAssetsCall;
use xprocess::Call as XAssetsProcessCall;
use xspot::Call as XSpotCall;
use xstaking::Call as XStakingCall;
use xtokens::Call as XTokensCall;

use Call;

pub trait CheckFee {
    fn check_fee(&self, switch: SwitchStore) -> Option<u64>;
}

impl CheckFee for Call {
    /// Return fee_power, which is part of the total_fee.
    /// total_fee = base_fee * fee_power + byte_fee * bytes
    ///
    /// fee_power = power_per_call
    fn check_fee(&self, switch: SwitchStore) -> Option<u64> {
        if switch.global {
            return None;
        };
        let base_power = match self {
            // xassets
            Call::XAssets(call) => match call {
                XAssetsCall::transfer(_, _, _, _) => Some(1),
                _ => None,
            },
            Call::XAssetsProcess(call) => match call {
                XAssetsProcessCall::withdraw(_, _, _, _) => Some(3),
                _ => None,
            },
            // xbridge
            Call::XBridgeOfBTC(call) => {
                let power = if switch.xbtc {
                    None
                } else {
                    match call {
                        BitcoinCall::push_header(_) => Some(10),
                        BitcoinCall::push_transaction(_) => Some(8),
                        BitcoinCall::create_withdraw_tx(_, _) => Some(5),
                        BitcoinCall::sign_withdraw_tx(_, _) => Some(5),
                        _ => None,
                    }
                };
                power
            }
            // xmining
            Call::XStaking(call) => match call {
                XStakingCall::register(_) => Some(100),
                XStakingCall::refresh(_, _, _, _) => Some(100),
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
            Call::XSpot(call) => {
                let power = if switch.spot {
                    None
                } else {
                    match call {
                        XSpotCall::put_order(_, _, _, _, _) => Some(8),
                        XSpotCall::cancel_order(_, _) => Some(2),
                        _ => None,
                    }
                };
                power
            }
            Call::Sudo(call) => match call {
                SudoCall::sudo(_) => Some(1),
                SudoCall::set_key(_) => Some(1),
                _ => None,
            },
            Call::XBridgeOfSDOT(call) => {
                let power = if switch.sdot {
                    None
                } else {
                    match call {
                        SdotCall::claim(_, _, _) => Some(2),
                        _ => None,
                    }
                };
                power
            }
            _ => None,
        };
        base_power
    }
}
