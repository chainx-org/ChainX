// Copyright 2018-2019 Chainpool.
use rstd::collections::btree_map::BTreeMap;

use xr_primitives::XString;

use xfee_manager::CallSwitcher;

use xassets::Call as XAssetsCall;
use xcontracts::Call as XContractsCall;

use crate::Call;

pub trait XContractsCheckFee {
    fn check_xcontracts_fee(
        &self,
        switcher: BTreeMap<CallSwitcher, bool>,
        method_weight_map: BTreeMap<XString, u64>,
    ) -> Option<u64>;
}

impl XContractsCheckFee for Call {
    fn check_xcontracts_fee(
        &self,
        switcher: BTreeMap<CallSwitcher, bool>,
        method_weight_map: BTreeMap<XString, u64>,
    ) -> Option<u64> {
        let get_switcher = |call_switcher: CallSwitcher| -> bool {
            switcher.get(&call_switcher).map(|b| *b).unwrap_or(false)
        };

        // Check if a certain emergency switch is on.
        if get_switcher(CallSwitcher::Global) {
            return None;
        };

        match self {
            Call::XContracts(..) if get_switcher(CallSwitcher::XContracts) => {
                return None;
            }
            _ => (),
        }

        match self {
            Call::XAssets(call) => match call {
                XAssetsCall::transfer(..) => {
                    get_method_call_weight_func!(method_weight_map, XAssets, transfer, 1)
                }
                _ => None,
            },
            Call::XContracts(call) => match call {
                XContractsCall::convert_to_asset(..) => Some(0),
                _ => None,
            },
            _ => None,
        }
    }
}
