// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::DispatchError;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait XGatewayBitcoinApi
    {
        fn verify_tx_valid(
            raw_tx: Vec<u8>,
            withdrawal_id_list: Vec<u32>,
            full_amount: bool,
        ) -> Result<bool, DispatchError>;
    }
}
