#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

use sp_std::collections::btree_map::BTreeMap;

pub use chainx_primitives::{AssetId, Precision};
pub use xpallet_assets::Chain;
pub use xpallet_gateway_records::{Withdrawal, WithdrawalState};

sp_api::decl_runtime_apis! {
    pub trait XGatewayRecordsApi<AccountId, Balance, BlockNumber> where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
    {
        fn withdrawal_list() -> BTreeMap<u32, Withdrawal<AccountId, Balance, BlockNumber>>;

        fn withdrawal_list_by_chain(chain: Chain) -> BTreeMap<u32, Withdrawal<AccountId, Balance, BlockNumber>>;
    }
}
