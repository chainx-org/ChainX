// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use crate::mock::ExtBuilder;

#[test]
fn base() {
    ExtBuilder::default().build().execute_with(|| {})
}
