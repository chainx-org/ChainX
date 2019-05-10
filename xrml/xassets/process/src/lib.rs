// Copyright 2018-2019 Chainpool.

//! this module is for funds-withdrawal

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

// Substrate
use primitives::traits::As;
use rstd::prelude::Vec;
use support::{decl_module, decl_storage, dispatch::Result};
use system::ensure_signed;

// ChainX
use xassets::{Chain, ChainT, Memo, Token};
use xr_primitives::AddrStr;
#[cfg(feature = "std")]
use xsupport::token;
use xsupport::{debug, warn};

pub trait Trait: xassets::Trait + xrecords::Trait + xbitcoin::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn withdraw(origin, token: Token, value: T::Balance, addr: AddrStr, ext: Memo) -> Result {
            let who = ensure_signed(origin)?;
            Self::check_black_list(&token)?;

            debug!("[withdraw]withdraw|who:{:?}|token:{:}|value:{:}", who, token!(token), value);

            let asset = xassets::Module::<T>::get_asset(&token)?;
            if asset.chain() == Chain::ChainX {
                return Err("Can't withdraw the asset on ChainX")
            }

            Self::verify_addr(&token, &addr, &ext)?;

            let min = Self::minimal_withdrawal_value(&token).expect("all token should has minimal withdrawal value");
            if value <= min {
                return Err("withdrawal value should larger than requirement")
            }

            xrecords::Module::<T>::withdrawal(&who, &token, value, addr, ext)?;
            Ok(())
        }

        fn revoke_withdraw(origin, id: u32) -> Result {
            let from = ensure_signed(origin)?;
            xrecords::Module::<T>::withdrawal_revoke(&from, id)
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Withdrawal {
        TokenBlackList get(token_black_list) config(): Vec<Token>;
    }
}

impl<T: Trait> Module<T> {
    fn check_black_list(token: &Token) -> Result {
        let list = Self::token_black_list();
        if list.contains(token) {
            warn!(
                "[check_black_list]|try asset:{:?}|current block list:{:?}",
                token!(token),
                list.into_iter()
                    .map(|item| token!(item))
                    .collect::<Vec<_>>()
            );
            return Err("this token is in blacklist");
        }
        Ok(())
    }

    fn verify_addr(token: &Token, addr: &[u8], _ext: &[u8]) -> Result {
        match token.as_slice() {
            <xbitcoin::Module<T> as ChainT>::TOKEN => xbitcoin::Module::<T>::check_addr(&addr, b""),
            _ => return Err("not found match token Token addr checker"),
        }
    }

    pub fn verify_address(token: Token, addr: AddrStr, ext: Memo) -> Result {
        Self::verify_addr(&token, &addr, &ext)
    }

    pub fn minimal_withdrawal_value(token: &Token) -> Option<T::Balance> {
        match token.as_slice() {
            <xbitcoin::Module<T> as ChainT>::TOKEN => {
                Some(As::sa(xbitcoin::Module::<T>::btc_withdrawal_fee()))
            }
            _ => None,
        }
    }
}
