#[frame_support::pallet]
#[allow(dead_code)]
pub mod pallet {
    use frame_support::traits::Hooks;
    use frame_system::pallet_prelude::BlockNumberFor;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl <T: Config> Pallet<T> {
        /// user request issue xbtc
        #[pallet::weight(0)]
        pub fn request_issue()
    }

