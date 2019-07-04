// Copyright 2018-2019 Chainpool.
use rstd::collections::btree_map::BTreeMap;

use xr_primitives::XString;

use xfee_manager::SwitchStore;

use xassets::Call as XAssetsCall;
use xbitcoin::lockup::Call as XBitcoinLockupCall;
use xbitcoin::Call as XBitcoinCall;
use xbridge_features::Call as XBridgeFeaturesCall;
use xfisher::Call as XFisherCall;
use xmultisig::Call as XMultiSigCall;
use xprocess::Call as XAssetsProcessCall;
use xsdot::Call as SdotCall;
use xspot::Call as XSpotCall;
use xstaking::Call as XStakingCall;
use xtokens::Call as XTokensCall;

use crate::Call;

pub trait CheckFee {
    fn check_fee(
        &self,
        switch: SwitchStore,
        method_weight_map: BTreeMap<XString, u64>,
    ) -> Option<u64>;
}

impl CheckFee for Call {
    /// Return fee_power, which is part of the total_fee.
    /// total_fee = base_fee * fee_power + byte_fee * bytes
    ///
    /// fee_power = power_per_call
    fn check_fee(
        &self,
        switch: SwitchStore,
        method_weight_map: BTreeMap<XString, u64>,
    ) -> Option<u64> {
        // MultiSigCall is on the top priority and can't be forbidden.
        if let Call::XMultiSig(call) = self {
            match call {
                XMultiSigCall::execute(..) => return Some(50),
                XMultiSigCall::confirm(..) => return Some(25),
                XMultiSigCall::remove_multi_sig_for(..) => return Some(1000),
                _ => (),
            }
        }

        // Check if a certain emergency switch is on.
        if switch.global {
            return None;
        };

        match self {
            Call::XSpot(..) if switch.spot => {
                return None;
            }
            Call::XBridgeOfBTC(..) if switch.xbtc => {
                return None;
            }
            Call::XBridgeOfSDOT(..) if switch.sdot => {
                return None;
            }
            _ => (),
        }

        macro_rules! get_method_call_weight {
            ($module:ty, $func:ty, $default:expr) => {
            {
                let method_weight_key = stringify!($module $func).as_bytes().to_vec();
                let method_weight = method_weight_map.get(&method_weight_key);
                Some(method_weight.map(|x| *x).unwrap_or($default))
            }
            };
        }

        macro_rules! match_method_call {
            (
                $(
                    $module:ident, $module_call:ident => (
                        $(
                            $method:ident : $default:expr,
                        )+
                    );
                )+
            ) => {
                match self {
                    $(
                        Call::$module(call) => match call {
                            $(
                                $module_call::$method(..) => get_method_call_weight!($module, $method, $default),
                            )+
                            _ => None,
                        },
                    )+
                    _ => None,
                }
            };
        }

        match_method_call! {

            XAssets, XAssetsCall => (
                transfer : 1,
            );

            XAssetsProcess, XAssetsProcessCall => (
                withdraw : 3,
                revoke_withdraw : 10,
            );

            XBridgeOfBTC, XBitcoinCall => (
                push_header : 10,
                push_transaction : 50,
                sign_withdraw_tx : 5,
                create_withdraw_tx : 5,
            );

            XBridgeOfBTCLockup, XBitcoinLockupCall => (
                push_transaction : 50,
            );

            XStaking, XStakingCall => (
                claim : 3,
                refresh : 10_000,
                nominate : 5,
                unfreeze : 2,
                register : 100_000,
                unnominate : 3,
                renominate : 800,
            );

            XTokens, XTokensCall => (
                claim : 3,
            );

            XSpot, XSpotCall => (
                put_order : 8,
                cancel_order : 2,
            );

            XBridgeOfSDOT, SdotCall => (
                claim : 2,
            );

            XBridgeFeatures, XBridgeFeaturesCall => (
                setup_bitcoin_trustee : 1000,
            );

            XFisher, XFisherCall => (
                report_double_signer : 5,
            );

        }
    }
}
