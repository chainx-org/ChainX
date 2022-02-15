// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use crate::{
    mock::{ExtBuilder, Test},
    Pallet, TrusteeSessionInfoLen,
};
use xp_assets_registrar::Chain;

#[test]
fn test_do_trustee_election() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(TrusteeSessionInfoLen::<Test>::get(Chain::Bitcoin), 0);

        assert_eq!(Pallet::<Test>::do_trustee_election(), Ok(()));

        assert_eq!(TrusteeSessionInfoLen::<Test>::get(Chain::Bitcoin), 1);
    })
}
