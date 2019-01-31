// Copyright 2018 Chainpool.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// for encode/decode
#[cfg(feature = "std")]
extern crate serde_derive;

// Needed for deriving `Encode` and `Decode` for `RawEvent`.
#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
// Needed for the set of mock primitives used in our tests.
#[cfg(test)]
extern crate substrate_primitives;

// for substrate runtime
// map!, vec! marco.
extern crate sr_std as rstd;

extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;

// for substrate runtime module lib
// Needed for type-safe access to storage DB.
#[macro_use]
extern crate srml_support as runtime_support;
extern crate srml_balances as balances;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;

extern crate xr_primitives;

// for chainx runtime module lib
extern crate xrml_xassets_assets as xassets;
extern crate xrml_xsupport as xsupport;

#[cfg(test)]
mod tests;

pub mod types;

use codec::Codec;
use rstd::prelude::*;
use runtime_support::dispatch::Result;

use runtime_support::StorageValue;

use xr_primitives::XString;

use xassets::{AssetType, Chain, ChainT, Memo, Token};
use xsupport::storage::linked_node::{LinkedNodeCollection, MultiNodeIndex, Node, NodeT};

pub type AddrStr = XString;

pub trait Trait: system::Trait + balances::Trait + xassets::Trait + timestamp::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as balances::Trait>::Balance,
        <T as timestamp::Trait>::Moment
    {
        Deposit(AccountId, Token, Balance),
        WithdrawalApply(u32, AccountId, Token, Balance, AddrStr, Memo, Moment),
        WithdrawalFinish(u32, bool),
    }
);

/// application for withdrawal
#[derive(PartialEq, Eq, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Application<AccountId, Balance, Moment> {
    id: u32,
    applicant: AccountId,
    token: Token,
    balance: Balance,
    addr: AddrStr,
    ext: Memo,
    time: Moment,
}

impl<AccountId, Balance, Moment> Application<AccountId, Balance, Moment>
where
    AccountId: Codec + Clone,
    Balance: Codec + Copy + Clone,
    Moment: Codec + Clone,
{
    fn new(
        id: u32,
        applicant: AccountId,
        token: Token,
        balance: Balance,
        addr: AddrStr,
        ext: Memo,
        time: Moment,
    ) -> Self {
        Application::<AccountId, Balance, Moment> {
            id,
            applicant,
            token,
            balance,
            addr,
            ext,
            time,
        }
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn applicant(&self) -> AccountId {
        self.applicant.clone()
    }
    pub fn token(&self) -> Token {
        self.token.clone()
    }
    pub fn balance(&self) -> Balance {
        self.balance
    }
    pub fn addr(&self) -> AddrStr {
        self.addr.clone()
    }
    pub fn ext(&self) -> Memo {
        self.ext.clone()
    }
    pub fn time(&self) -> Moment {
        self.time.clone()
    }
}

impl<AccountId, Balance, Moment> NodeT for Application<AccountId, Balance, Moment> {
    type Index = u32;
    fn index(&self) -> Self::Index {
        self.id
    }
}

pub struct LinkedMultiKey<T: Trait>(runtime_support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedMultiKey<T> {
    type Header = ApplicationMHeader<T>;
    type NodeMap = ApplicationMap<T>;
    type Tail = ApplicationMTail<T>;
}

decl_storage! {
    trait Store for Module<T: Trait> as XAssetsRecords {
        /// linked node header
        pub ApplicationMHeader get(application_mheader): map Chain => Option<MultiNodeIndex<Chain, Application<T::AccountId, T::Balance, T::Moment>>>;
        /// linked node tail
        pub ApplicationMTail get(application_mtail): map Chain => Option<MultiNodeIndex<Chain, Application<T::AccountId, T::Balance, T::Moment>>>;
        /// withdrawal applications collection, use serial number to mark them, and has prev and next to link them
        pub ApplicationMap get(application_map): map u32 => Option<Node<Application<T::AccountId, T::Balance, T::Moment>>>;
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

        let free = xassets::Module::<T>::free_balance(who, token);
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
        xassets::Module::<T>::issue(token, who, balance)?;
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
        let appl = Application::<T::AccountId, T::Balance, T::Moment>::new(
            id,
            who.clone(),
            token.clone(),
            balance,
            addr,
            ext,
            timestamp::Module::<T>::now(),
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
            appl.token,
            appl.balance,
            appl.addr,
            appl.ext,
            appl.time,
        ));
        Ok(())
    }

    /// withdrawal finish, let the locking token destroy
    pub fn withdrawal_finish(serial_number: u32, success: bool) -> Result {
        let mut node = if let Some(node) = Self::application_map(serial_number) {
            node
        } else {
            return Err("withdrawal application record not exist");
        };

        let asset = xassets::Module::<T>::get_asset(&node.data.token())?;

        node.remove_option_with_key::<LinkedMultiKey<T>, Chain>(asset.chain())?;

        let application = node.data;
        let who = application.applicant();
        let token = application.token();
        let balance = application.balance();
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
        xassets::Module::<T>::move_balance(
            token,
            who,
            AssetType::Free,
            who,
            AssetType::ReservedWithdrawal,
            value,
        )
        .map_err(|e| e.info())
    }

    fn unlock(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        xassets::Module::<T>::move_balance(
            token,
            who,
            AssetType::ReservedWithdrawal,
            who,
            AssetType::Free,
            value,
        )
        .map_err(|e| e.info())
    }

    fn destroy(who: &T::AccountId, token: &Token, value: T::Balance) -> Result {
        xassets::Module::<T>::destroy(&token, &who, value)
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
    ) -> Vec<Application<T::AccountId, T::Balance, T::Moment>> {
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
