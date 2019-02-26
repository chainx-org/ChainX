// Copyright 2018 Chainpool.

//! this module is for chainx system

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use parity_codec_derive::Decode;
use parity_codec_derive::Encode;

// for substrate
use sr_std as rstd;
use substrate_inherents as inherents;

use srml_support::{decl_module, decl_storage, dispatch::Result, StorageValue};
use srml_system as system;

use xrml_xsupport::{error, info};

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
use inherents::ProvideInherentData;
use inherents::{
    InherentData, InherentIdentifier, IsFatalError, MakeFatalError, ProvideInherent, RuntimeString,
};
use rstd::prelude::Vec;
use rstd::result::Result as StdResult;

use system::ensure_inherent;

pub trait Trait: system::Trait {
    type ValidatorList: ValidatorList<Self::AccountId>;
    type Validator: Validator<Self::AccountId>;
}

pub trait ValidatorList<AccountId> {
    fn validator_list() -> Vec<AccountId>;
}

pub trait Validator<AccountId> {
    fn get_validator_by_name(name: &[u8]) -> Option<AccountId>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_block_producer(origin, producer: T::AccountId) -> Result {
            ensure_inherent(origin)?;
            info!("height:{:}, blockproducer: {:}", system::Module::<T>::block_number(), producer);

            if Self::is_validator(&producer) == false {
                error!("producer:{:} not in current validators!, validators is:{:?}", producer, T::ValidatorList::validator_list());
                panic!("producer not in current validators!");
            }

            BlockProducer::<T>::put(producer);
            Ok(())
        }
        fn on_finalise(_n: T::BlockNumber) {
            BlockProducer::<T>::kill();
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XSystem {
        pub BlockProducer get(block_producer): Option<T::AccountId>;
        pub DeathAccount get(death_account) config(): T::AccountId;
        // TODO remove this to other module
        pub BurnAccount get(burn_account) config(): T::AccountId;
    }
}

impl<T: Trait> Module<T> {
    fn is_validator(producer: &T::AccountId) -> bool {
        let validators = T::ValidatorList::validator_list();
        validators.contains(&producer)
    }
}

impl<T: Trait> ProvideInherent for Module<T> {
    type Call = Call<T>;
    type Error = MakeFatalError<RuntimeString>;
    const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;
    fn create_inherent(data: &InherentData) -> Option<Self::Call> {
        let r = data
            .get_data::<Vec<u8>>(&INHERENT_IDENTIFIER)
            .expect("gets and decodes producer inherent data");
        let producer_name = r.expect("producer must set before");

        let producer: T::AccountId = if let Some(a) =
            T::Validator::get_validator_by_name(&producer_name)
        {
            a
        } else {
            error!("[create_inherent] producer_name:{:} do not have accountid on chain, may not be registerd or do not have current storage", std::str::from_utf8(&producer_name).unwrap_or(&format!("{:?}", producer_name)));
            panic!("[create_inherent] do not have accountid on chain, may not be registerd or do not have current storage");
        };

        if !Self::is_validator(&producer) {
            error!(
                "[create_inherent] producer_name:{:?}, producer:{:} not in current validators!, validators is:{:?}",
                std::str::from_utf8(&producer_name).unwrap_or(&format!("{:?}", producer_name)),
                producer,
                T::ValidatorList::validator_list()
            );
            panic!("[create_inherent] producer not in current validators!");
        }

        Some(Call::set_block_producer(producer))
    }

    fn check_inherent(call: &Self::Call, _data: &InherentData) -> StdResult<(), Self::Error> {
        let producer = match call {
            Call::set_block_producer(ref p) => p.clone(),
            _ => return Err(RuntimeString::from("not found producer in call").into()),
        };

        if !Self::is_validator(&producer) {
            error!(
                "[check_inherent] producer:{:} not in current validators!, validators is:{:?}",
                producer,
                T::ValidatorList::validator_list()
            );
            return Err(
                RuntimeString::from("[check_inherent] producer not in current validators").into(),
            );
        }
        Ok(())
    }
}

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"producer";

#[derive(Encode)]
#[cfg_attr(feature = "std", derive(Debug, Decode))]
pub enum InherentError {
    /// no producer set
    NoBlockProducer,
    /// Some other error.
    Other(RuntimeString),
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match self {
            _ => true,
        }
    }
}

#[cfg(feature = "std")]
pub struct InherentDataProvider {
    block_producer_name: Vec<u8>,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
    pub fn new(producer_name: Vec<u8>) -> Self {
        InherentDataProvider {
            block_producer_name: producer_name,
        }
    }
}

#[cfg(feature = "std")]
impl ProvideInherentData for InherentDataProvider {
    fn inherent_identifier(&self) -> &'static InherentIdentifier {
        &INHERENT_IDENTIFIER
    }

    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> StdResult<(), RuntimeString> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.block_producer_name)
    }

    fn error_to_string(&self, _error: &[u8]) -> Option<String> {
        // do not handle due no check for this inherent
        None
    }
}
