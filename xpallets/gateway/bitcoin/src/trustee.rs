
use frame_support::dispatch::DispatchError;

use btc_keys::Address;

use crate::types::TrusteeAddrInfo;
use crate::Trait;

pub fn trustee_session<T: Trait>(
) -> Result<TrusteeSessionInfo<T::AccountId, TrusteeAddrInfo>, DispatchError> {
    T::TrusteeSessionProvider::current_trustee_session()
}

#[inline]
fn trustee_addr_info_pair<T: Trait>() -> Result<(TrusteeAddrInfo, TrusteeAddrInfo), DispatchError> {
    T::TrusteeSessionProvider::current_trustee_session()
        .map(|session_info| (session_info.hot_address, session_info.cold_address))
}

#[inline]
pub fn get_trustee_address_pair<T: Trait>() -> Result<(Address, Address), DispatchError> {
    trustee_addr_info_pair::<T>().map(|(hot_info, cold_info)| (hot_info.addr, cold_info.addr))
}

#[inline]
pub fn get_last_trustee_address_pair<T: Trait>() -> Result<(Address, Address), DispatchError> {
    T::TrusteeSessionProvider::last_trustee_session().map(|session_info| {
        (
            session_info.hot_address.addr,
            session_info.cold_address.addr,
        )
    })
}

pub fn get_hot_trustee_address<T: Trait>() -> Result<Address, DispatchError> {
    trustee_addr_info_pair::<T>().map(|(addr_info, _)| addr_info.addr)
}

pub fn get_hot_trustee_redeem_script<T: Trait>() -> Result<Script, DispatchError> {
    trustee_addr_info_pair::<T>().map(|(addr_info, _)| addr_info.redeem_script.into())
}


// /// Get the required number of signatures
// /// sig_num: Number of signatures required
// /// trustee_num: Total number of multiple signatures
// /// NOTE: Signature ratio greater than 2/3
// pub fn get_sig_num<T: Trait>() -> (u32, u32) {
//     let trustee_list = T::TrusteeSessionProvider::current_trustee_session()
//         .map(|session_info| session_info.trustee_list)
//         .expect("the trustee_list must exist; qed");
//     let trustee_num = trustee_list.len() as u32;
//     (two_thirds_unsafe(trustee_num), trustee_num)
// }