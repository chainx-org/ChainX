use frame_support::weights::Weight;

pub trait WeightInfo {
    fn claim() -> Weight;
    fn set_claim_staking_requirement() -> Weight;
    fn set_claim_frequency_limit() -> Weight;
    fn set_asset_power() -> Weight;
}

impl WeightInfo for () {
    fn claim() -> Weight {
        1_000_000_000
    }
    fn set_claim_staking_requirement() -> Weight {
        1_000_000_000
    }
    fn set_claim_frequency_limit() -> Weight {
        1_000_000_000
    }
    fn set_asset_power() -> Weight {
        1_000_000_000
    }
}
