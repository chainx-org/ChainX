// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

mod mock;

use mock::*;

#[test]
fn base() {
    ExtBuilder::default().build().execute_with(|| {})
}
