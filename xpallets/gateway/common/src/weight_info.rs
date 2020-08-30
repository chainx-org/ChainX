use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn withdraw() -> Weight;
    fn setup_trustee() -> Weight;
    fn transition_trustee_session(u: u32) -> Weight;
    fn set_withdrawal_state() -> Weight;
    fn set_trustee_info_config() -> Weight;
    fn force_set_binding() -> Weight;
}

impl WeightInfo for () {
    fn withdraw() -> Weight {
        (1474990000 as Weight)
            .saturating_add(DbWeight::get().reads(14 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn setup_trustee() -> Weight {
        (198467000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn transition_trustee_session(u: u32) -> Weight {
        (1215766000 as Weight)
            .saturating_add((326000 as Weight).saturating_mul(u as Weight))
            .saturating_add(DbWeight::get().reads(8 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
    fn set_withdrawal_state() -> Weight {
        (819458000 as Weight)
            .saturating_add(DbWeight::get().reads(15 as Weight))
            .saturating_add(DbWeight::get().writes(8 as Weight))
    }
    fn set_trustee_info_config() -> Weight {
        (19710000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn force_set_binding() -> Weight {
        (94894000 as Weight)
            .saturating_add(DbWeight::get().reads(4 as Weight))
            .saturating_add(DbWeight::get().writes(3 as Weight))
    }
}
