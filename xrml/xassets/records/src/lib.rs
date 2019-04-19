// Copyright 2018-2019 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
pub mod types;

// Substrate
use rstd::prelude::*;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageValue};

// ChainX
use xassets::{AssetType, Chain, ChainT, Memo, Token};
use xsupport::storage::linked_node::{MultiNodeIndex, Node};

use xsupport::{error, info};
#[cfg(feature = "std")]
use xsupport::{token, u8array_to_addr, u8array_to_string};

pub use self::types::{AddrStr, Application, HeightOrTime, LinkedMultiKey, RecordInfo, TxState};

pub trait Trait: system::Trait + xassets::Trait + timestamp::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;
        // only for root
        fn deposit_from_root(who: T::AccountId, token: Token, balance: T::Balance) -> Result {
            Self::deposit(&who, &token, balance)
        }

        fn withdrawal_from_root(who: T::AccountId, token: Token, balance: T::Balance) -> Result {
            Self::withdrawal(&who, &token, balance, Default::default(), Default::default())
        }

        fn withdrawal_finish_from_root(withdrawal_id: u32, success: bool) -> Result {
            match Self::withdrawal_finish(withdrawal_id, success) {
                Ok(_) => {
                    info!("[withdraw]|ID of withdrawal completion: {:}", withdrawal_id);
                    Ok(())
                }
                Err(_e) => {
                    error!("[withdraw]|ID of withdrawal ERROR! {:}, reason:{:}, please use root to fix it", withdrawal_id, _e);
                    Err(_e)
                }
            }
        }

        pub fn fix_withdrawal_state_list(item: Vec<(u32, bool)>) -> Result {
            for (withdrawal_id, success) in item {
                let _ = Self::withdrawal_finish_from_root(withdrawal_id, success);
            }
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as xassets::Trait>::Balance {
        Deposit(AccountId, Token, Balance),
        WithdrawalApply(u32, AccountId, Chain, Token, Balance, Memo, AddrStr, TxState),
        WithdrawalFinish(u32, bool),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as XAssetsRecords {
        /// linked node header
        pub ApplicationMHeader get(application_mheader): map Chain => Option<MultiNodeIndex<Chain, Application<T::AccountId, T::Balance, T::BlockNumber>>>;
        /// linked node tail
        pub ApplicationMTail get(application_mtail): map Chain => Option<MultiNodeIndex<Chain, Application<T::AccountId, T::Balance, T::BlockNumber>>>;
        /// withdrawal applications collection, use serial number to mark them, and has prev and next to link them
        pub ApplicationMap get(application_map): map u32 => Option<Node<Application<T::AccountId, T::Balance, T::BlockNumber>>>;
        /// withdrawal application serial number
        pub SerialNumber get(number): u32 = 0;
    }
}

impl<T: Trait> Module<T> {
    /// deposit/withdrawal pre-process
    fn before(_: &T::AccountId, token: &Token) -> Result {
        if token.as_slice() == <xassets::Module<T> as ChainT>::TOKEN {
            return Err("can't deposit/withdrawal chainx token");
        }
        // other check
        Ok(())
    }

    fn withdraw_check_before(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        Self::before(who, token)?;

        let free = xassets::Module::<T>::free_balance_of(who, token);
        if free < value {
            return Err("free balance not enough for this account");
        }

        Ok(())
    }
}

impl<T: Trait> Module<T> {
    /// deposit, notice this func has include deposit_init and deposit_finish (not wait for block confirm process)
    pub fn deposit(who: &T::AccountId, token: &Token, balance: T::Balance) -> Result {
        Self::before(who, token)?;

        info!(
            "[deposit]|who:{:?}|token:{:}|balance:{:}",
            who,
            token!(token),
            balance
        );

        let _ = xassets::Module::<T>::issue(token, who, balance)?;
        Self::deposit_event(RawEvent::Deposit(who.clone(), token.clone(), balance));
        Ok(())
    }

    /// withdrawal, notice this func has include withdrawal_init and withdrawal_locking
    pub fn withdrawal(
        who: &T::AccountId,
        token: &Token,
        balance: T::Balance,
        addr: AddrStr,
        ext: Memo,
    ) -> Result {
        Self::withdraw_check_before(who, token, balance)?;

        let asset = xassets::Module::<T>::get_asset(token)?;

        let id = Self::number();

        info!(
            "[withdrawal]|id:{:}|who:{:?}|token:{:}|balance:{:}|addr:{:}|memo:{:}",
            id,
            who,
            token!(token),
            balance,
            u8array_to_addr(&addr),
            u8array_to_string(&ext)
        );

        let appl = Application::<T::AccountId, T::Balance, T::BlockNumber>::new(
            id,
            who.clone(),
            token.clone(),
            balance,
            addr,
            ext,
            system::Module::<T>::block_number(),
        );

        let n = Node::new(appl.clone());
        n.init_storage_with_key::<LinkedMultiKey<T>, Chain>(asset.chain());
        // set from tail
        if let Some(tail) = Self::application_mtail(asset.chain()) {
            let index = tail.index();
            if let Some(mut node) = Self::application_map(index) {
                // reserve token, wait to destroy
                Self::lock(who, token, balance)?;
                node.add_option_after_with_key::<LinkedMultiKey<T>, Chain>(n, asset.chain())?;
            }
        }

        let newid = match id.checked_add(1_u32) {
            Some(r) => r,
            None => 0,
        };
        SerialNumber::<T>::put(newid);

        Self::deposit_event(RawEvent::WithdrawalApply(
            appl.id,
            appl.applicant,
            asset.chain(),
            appl.token,
            appl.balance,
            appl.ext,
            appl.addr, // if btc, the addr is base58 addr
            TxState::Applying,
        ));
        Ok(())
    }

    /// withdrawal finish, let the locking token destroy
    pub fn withdrawal_finish(serial_number: u32, success: bool) -> Result {
        let mut node = if let Some(node) = Self::application_map(serial_number) {
            node
        } else {
            error!("[withdrawal_finish]|withdrawal application record not exist|withdrawal id:{:}|success:{:}", serial_number, success);
            return Err("withdrawal application record not exist");
        };

        let asset = xassets::Module::<T>::get_asset(&node.data.token())?;

        node.remove_option_with_key::<LinkedMultiKey<T>, Chain>(asset.chain())?;

        let application = node.data;
        let who = application.applicant();
        let token = application.token();
        let balance = application.balance();

        info!(
            "[withdrawal_finish]|wirhdrawal id:{:}|who:{:?}|token:{:}|balance:{:}",
            serial_number,
            who,
            token!(token),
            balance
        );
        // destroy reserved token
        if success {
            Self::destroy(&who, &token, balance)?;
        } else {
            Self::unlock(&who, &token, balance)?;
        }

        Self::deposit_event(RawEvent::WithdrawalFinish(serial_number, success));
        Ok(())
    }

    fn lock(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        let _ = xassets::Module::<T>::move_balance(
            token,
            who,
            AssetType::Free,
            who,
            AssetType::ReservedWithdrawal,
            value,
        )
        .map_err(|e| e.info())?;
        Ok(())
    }

    fn unlock(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        let _ = xassets::Module::<T>::move_balance(
            token,
            who,
            AssetType::ReservedWithdrawal,
            who,
            AssetType::Free,
            value,
        )
        .map_err(|e| e.info())?;
        Ok(())
    }

    fn destroy(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        let _ = xassets::Module::<T>::destroy(&token, &who, value)?;
        Ok(())
    }

    pub fn withdrawal_application_numbers(chain: Chain, max_count: u32) -> Option<Vec<u32>> {
        let mut vec = Vec::new();
        // begin from header
        if let Some(header) = Self::application_mheader(chain) {
            let mut index = header.index();
            for _ in 0..max_count {
                if let Some(node) = Self::application_map(&index) {
                    vec.push(node.index());
                    if let Some(next) = node.next() {
                        index = next;
                    } else {
                        return Some(vec);
                    }
                }
            }
            return Some(vec);
        }
        None
    }

    pub fn withdrawal_applications(
        chain: Chain,
    ) -> Vec<Application<T::AccountId, T::Balance, T::BlockNumber>> {
        let mut vec = Vec::new();
        // begin from header
        if let Some(header) = Self::application_mheader(chain) {
            let mut index = header.index();
            while let Some(node) = Self::application_map(&index) {
                let next = node.next().clone();
                vec.push(node.data);
                if let Some(next) = next {
                    index = next;
                } else {
                    break;
                }
            }
        }
        vec
    }
}
