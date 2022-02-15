// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]

use sp_std::collections::btree_map::BTreeMap;

use codec::Codec;

pub use sherpax_primitives::{AssetId, Decimals};
pub use xp_assets_registrar::Chain;
pub use xpallet_gateway_records::{Withdrawal, WithdrawalRecordId, WithdrawalState};

sp_api::decl_runtime_apis! {
    pub trait XGatewayRecordsApi<AccountId, Balance, BlockNumber>
    where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
    {
        fn withdrawal_list() -> BTreeMap<WithdrawalRecordId, Withdrawal<AccountId, AssetId, Balance, BlockNumber>>;

        fn withdrawal_list_by_chain(chain: Chain) -> BTreeMap<WithdrawalRecordId, Withdrawal<AccountId, AssetId, Balance, BlockNumber>>;
    }
}
