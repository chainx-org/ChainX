// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub mod assets;
pub mod bitcoin;

use xp_genesis_builder::FullParams;

use chainx_primitives::{AccountId, Balance};

pub fn genesis_builder_params() -> FullParams<AccountId, Balance, Balance, Balance> {
    serde_json::from_str(include_str!("../res/3696796_regenesis_params.json"))
        .map_err(|e| log::error!("{:?}", e))
        .expect("JSON was not well-formatted")
}
