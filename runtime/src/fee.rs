// Copyright 2018 Chainpool

use xfee_manager::SwitchStore;

use sudo::Call as SudoCall;
use xassets::Call as XAssetsCall;
use xbitcoin::Call as XBitcoinCall;
use xmultisig::Call as XMultiSigCall;
use xprocess::Call as XAssetsProcessCall;
use xsdot::Call as SdotCall;
use xspot::Call as XSpotCall;
use xstaking::Call as XStakingCall;
use xtokens::Call as XTokensCall;

use crate::Call;

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
                        XBitcoinCall::push_header(_) => Some(10),
                        XBitcoinCall::push_transaction(_) => Some(8),
                        XBitcoinCall::create_withdraw_tx(_, _) => Some(5),
                        XBitcoinCall::sign_withdraw_tx(_) => Some(5),
                        _ => None,
                    }
                };
                power
            }
            // xmining
            Call::XStaking(call) => match call {
                XStakingCall::register(_) => Some(1000),
                XStakingCall::refresh(_, _, _, _) => Some(1000),
                XStakingCall::nominate(_, _, _) => Some(5),
                XStakingCall::unnominate(_, _, _) => Some(3),
                XStakingCall::unfreeze(_, _) => Some(2),
                XStakingCall::claim(_) => Some(3),
                XStakingCall::setup_trustee(_, _, _, _) => Some(1000),
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
            Call::XMultiSig(call) => match call {
                XMultiSigCall::deploy(_, _) => Some(100),
                XMultiSigCall::execute(_, _) => Some(10),
                XMultiSigCall::confirm(_, _) => Some(5),
                XMultiSigCall::is_owner_for(_) => Some(1),
                XMultiSigCall::remove_multi_sig_for(_, _) => Some(5),
                _ => None,
            },
            _ => None,
        };
        base_power
    }
}
