// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

//! Runtime API definition required by ChainX RPC extensions.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::DispatchError;
use sp_std::vec::Vec;
pub use xpallet_gateway_bitcoin::{types::BtcHeaderInfo, BtcHeader, BtcWithdrawalProposal, H256};

sp_api::decl_runtime_apis! {
    pub trait XGatewayBitcoinApi<AccountId>
        where AccountId: codec::Codec
    {
        fn verify_tx_valid(
            raw_tx: Vec<u8>,
            withdrawal_id_list: Vec<u32>,
            full_amount: bool,
        ) -> Result<bool, DispatchError>;

        fn get_withdrawal_proposal() -> Option<BtcWithdrawalProposal<AccountId>>;

        fn get_genesis_info() -> (BtcHeader, u32);

        fn get_btc_block_header(txid: H256) -> Option<BtcHeaderInfo>;
    }
}
