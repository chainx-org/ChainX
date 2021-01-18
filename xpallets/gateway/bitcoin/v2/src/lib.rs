// Copyright 2021 ChainX Project Authors. Licensed under GPL-3.0.

//! This module implements Bitcoin Bridge V2.

#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod vault {
    use frame_support::traits::{Currency, LockableCurrency};
    use frame_support::{pallet_prelude::*, storage::types::ValueQuery};
    use frame_system::pallet_prelude::*;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a vault.
        #[pallet::weight(0)]
        fn register_vault(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin);
            Ok(().into())
        }
    }

    #[pallet::type_value]
    pub(super) fn DefaultCollateral<T: Config>() -> BalanceOf<T> {
        0.into()
    }

    #[pallet::storage]
    pub(super) type TotalCollateral<T: Config> =
        StorageValue<_, BalanceOf<T>, ValueQuery, DefaultCollateral<T>>;
}

#[frame_support::pallet]
// NOTE: Example is name of the pallet, it will be used as unique identifier for storage
pub mod pallet {
    use frame_support::pallet_prelude::*; // Import various types used in pallet definition
    use frame_system::pallet_prelude::*; // OriginFor helper type for implementing dispatchables.

    type BalanceOf<T> = <T as Config>::Balance;

    // Define the generic parameter of the pallet
    // The macro checks trait generics: is expected none or `I = ()`.
    // The macro parses `#[pallet::constant]` attributes: used to generate constant metadata,
    // expected syntax is `type $IDENT: Get<$TYPE>;`.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Balance: Parameter + From<u8>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    // Define some additional constant to put into the constant metadata.
    #[pallet::extra_constants]
    impl<T: Config> Pallet<T> {
        /// Some description
        fn exra_constant_name() -> u128 {
            4u128
        }
    }

    // Define the pallet struct placeholder, various pallet function are implemented on it.
    // The macro checks struct generics: is expected `T` or `T, I = DefaultInstance`
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    // Implement on the pallet hooks on pallet.
    // The macro checks:
    // * trait is `Hooks` (imported from pallet_prelude)
    // * struct is `Pallet<T>` or `Pallet<T, I>`
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Declare Call struct and implement dispatchables.
    //
    // WARNING: Each parameter used in functions must implement: Clone, Debug, Eq, PartialEq,
    // Codec.
    //
    // The macro checks:
    // * pallet is `Pallet<T>` or `Pallet<T, I>`
    // * trait is `Call`
    // * each dispatchable functions first argument is `origin: OriginFor<T>` (OriginFor is
    //   imported from frame_system.
    //
    // The macro parse `#[pallet::compact]` attributes, function parameter with this attribute
    // will be encoded/decoded using compact codec in implementation of codec for the enum
    // `Call`.
    //
    // The macro generate the enum `Call` with a variant for each dispatchable and implements
    // codec, Eq, PartialEq, Clone and Debug.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Doc comment put in metadata
        #[pallet::weight(0)] // Defines weight for call (function parameters are in scope)
        fn toto(origin: OriginFor<T>, #[pallet::compact] _foo: u32) -> DispatchResultWithPostInfo {
            let _ = origin;
            unimplemented!();
        }
    }

    // Declare pallet Error enum. (this is optional)
    // The macro checks enum generics and that each variant is unit.
    // The macro generate error metadata using doc comment on each variant.
    #[pallet::error]
    pub enum Error<T> {
        /// doc comment put into metadata
        InsufficientProposersBalance,
    }

    // Declare pallet Event enum. (this is optional)
    //
    // WARNING: Each type used in variants must implement: Clone, Debug, Eq, PartialEq, Codec.
    //
    // The macro generates event metadata, and derive Clone, Debug, Eq, PartialEq and Codec
    #[pallet::event]
    // Additional argument to specify the metadata to use for given type.
    #[pallet::metadata(BalanceOf<T> = "Balance", u32 = "Other")]
    // Generate a funciton on Pallet to deposit an event.
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// doc comment put in metadata
        // `<T as frame_system::Config>::AccountId` is not defined in metadata list, the last
        // Thus the metadata is `<T as frame_system::Config>::AccountId`.
        Proposed(<T as frame_system::Config>::AccountId),
        /// doc
        // here metadata will be `Balance` as define in metadata list
        Spending(BalanceOf<T>),
        // here metadata will be `Other` as define in metadata list
        Something(u32),
    }

    // Define a struct which implements `frame_support::traits::Get<T::Balance>`
    #[pallet::type_value]
    pub(super) fn MyDefault<T: Config>() -> T::Balance {
        3.into()
    }

    // Declare a storage, any amount of storage can be declared.
    //
    // Is expected either `StorageValue`, `StorageMap` or `StorageDoubleMap`.
    // The macro generates for struct `$identP` (for storage of name `$ident`) and implement
    // storage instance on it.
    // The macro macro expand the metadata for the storage with the type used:
    // * For storage value the type for value will be copied into metadata
    // * For storage map the type for value and the type for key will be copied into metadata
    // * For storage double map the type for value, key1, and key2 will be copied into
    //   metadata.
    //
    // NOTE: for storage hasher, the type is not copied because storage hasher trait already
    // implements metadata. Thus generic storage hasher is supported.
    #[pallet::storage]
    pub(super) type MyStorageValue<T: Config> =
        StorageValue<_, T::Balance, ValueQuery, MyDefault<T>>;

    // Another declaration
    #[pallet::storage]
    #[pallet::getter(fn my_storage)]
    pub(super) type MyStorage<T> = StorageMap<_, Blake2_128Concat, u32, u32>;

    // Declare genesis config. (This is optional)
    //
    // The macro accept either type alias or struct or enum, it checks generics are consistent.
    //
    // Type must implement `Default` traits
    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig {
        _myfield: u32,
    }

    // Declare genesis builder. (This is need only if GenesisConfig is declared)
    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {}
    }

    // Declare a pallet origin. (this is optional)
    //
    // The macro accept type alias or struct or enum, it checks generics are consistent.
    #[pallet::origin]
    pub struct Origin<T>(PhantomData<T>);

    // Declare validate_unsigned implementation.
    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;
        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
        }
    }

    // Declare inherent provider for pallet. (this is optional)
    //
    // The macro checks pallet is `Pallet<T>` or `Pallet<T, I>` and trait is `ProvideInherent`
    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        type Error = InherentError;

        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
            unimplemented!();
        }
    }

    // Regular rust code needed for implementing ProvideInherent trait

    #[derive(codec::Encode, sp_runtime::RuntimeDebug)]
    #[cfg_attr(feature = "std", derive(codec::Decode))]
    pub enum InherentError {}

    impl sp_inherents::IsFatalError for InherentError {
        fn is_fatal_error(&self) -> bool {
            unimplemented!();
        }
    }

    pub const INHERENT_IDENTIFIER: sp_inherents::InherentIdentifier = *b"testpall";
}
