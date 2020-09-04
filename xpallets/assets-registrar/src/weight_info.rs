use frame_support::weights::Weight;

pub trait WeightInfo {
    fn register() -> Weight;
    fn deregister() -> Weight;
    fn recover() -> Weight;
    fn update_asset_info() -> Weight;
}

impl WeightInfo for () {
    fn register() -> Weight {
        1_000_000_000
    }
    fn deregister() -> Weight {
        1_000_000_000
    }
    fn recover() -> Weight {
        1_000_000_000
    }
    fn update_asset_info() -> Weight {
        1_000_000_000
    }
}
