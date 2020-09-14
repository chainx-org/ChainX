// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::weights::Weight;

pub trait WeightInfo {
    fn put_order() -> Weight;
    fn cancel_order() -> Weight;
    fn force_cancel_order() -> Weight;
    fn set_handicap() -> Weight;
    fn set_price_fluctuation() -> Weight;
    fn add_trading_pair() -> Weight;
    fn update_trading_pair() -> Weight;
}

impl WeightInfo for () {
    fn put_order() -> Weight {
        1_000_000_000
    }
    fn cancel_order() -> Weight {
        1_000_000_000
    }
    fn force_cancel_order() -> Weight {
        1_000_000_000
    }
    fn set_handicap() -> Weight {
        1_000_000_000
    }
    fn set_price_fluctuation() -> Weight {
        1_000_000_000
    }
    fn add_trading_pair() -> Weight {
        1_000_000_000
    }
    fn update_trading_pair() -> Weight {
        1_000_000_000
    }
}
