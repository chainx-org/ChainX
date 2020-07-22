use frame_support::dispatch::DispatchError;

use btc_crypto::dhash160;
use btc_keys::{Address, Public, Type};
use btc_primitives::Bytes;
use btc_script::{Builder, Opcode, Script};

use xpallet_gateway_common::{
    traits::TrusteeSession, trustees::bitcoin::BTCTrusteeAddrInfo, types::TrusteeSessionInfo,
};
use xpallet_support::error;

use crate::{Module, Trait};

pub fn trustee_session<T: Trait>(
) -> Result<TrusteeSessionInfo<T::AccountId, BTCTrusteeAddrInfo>, DispatchError> {
    T::TrusteeSessionProvider::current_trustee_session()
}

#[inline]
fn trustee_addr_info_pair<T: Trait>(
) -> Result<(BTCTrusteeAddrInfo, BTCTrusteeAddrInfo), DispatchError> {
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

pub fn create_multi_address<T: Trait>(
    pubkeys: &Vec<Public>,
    sig_num: u32,
) -> Option<BTCTrusteeAddrInfo> {
    let sum = pubkeys.len() as u32;
    if sig_num > sum {
        panic!("required sig num should less than trustee_num; qed")
    }
    if sum > 15 {
        error!("bitcoin's multisig can't more than 15, current is:{:}", sum);
        return None;
    }

    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sig_num as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let mut build = Builder::default().push_opcode(opcode);
    for pubkey in pubkeys.iter() {
        build = build.push_bytes(&pubkey);
    }

    let opcode = match Opcode::from_u8(Opcode::OP_1 as u8 + sum as u8 - 1) {
        Some(o) => o,
        None => return None,
    };
    let redeem_script = build
        .push_opcode(opcode)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();

    let addr = Address {
        kind: Type::P2SH,
        network: Module::<T>::network_id(),
        hash: dhash160(&redeem_script),
    };
    let script_bytes: Bytes = redeem_script.into();
    Some(BTCTrusteeAddrInfo {
        addr,
        redeem_script: script_bytes.into(),
    })
}
