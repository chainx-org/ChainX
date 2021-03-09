#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]

use sp_std::vec::Vec;

use codec::Codec;

sp_api::decl_runtime_apis! {
    pub trait XGatewayBitcoinV2Api<AccountId, BlockNumber, Balance>
        where AccountId:Codec, BlockNumber: Codec, Balance: Codec
    {
        fn get_first_matched_vault(xbtc_amount: Balance) -> Option<(AccountId, Vec<u8>)>;
    }
}
