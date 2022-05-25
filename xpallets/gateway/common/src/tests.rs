use frame_system::RawOrigin;

use crate::{
    mock::{bob, charlie, dave, ExtBuilder, Test, XAssets, XGatewayCommon, XGatewayRecords},
    Pallet, TrusteeSessionInfoLen, TrusteeSessionInfoOf, TrusteeSigRecord,
};
use frame_support::assert_ok;
use xp_assets_registrar::Chain;
use xp_protocol::X_BTC;

#[test]
fn test_do_trustee_election() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(TrusteeSessionInfoLen::<Test>::get(Chain::Bitcoin), 0);

        assert_eq!(Pallet::<Test>::do_trustee_election(Chain::Bitcoin), Ok(()));

        assert_eq!(TrusteeSessionInfoLen::<Test>::get(Chain::Bitcoin), 1);
    })
}

#[test]
fn test_move_trustee_into_little_black_house() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(XGatewayCommon::do_trustee_election(Chain::Bitcoin), Ok(()));
        assert!(!XGatewayCommon::trustee_transition_status(Chain::Bitcoin));

        TrusteeSigRecord::<Test>::mutate(Chain::Bitcoin, bob(), |record| *record = 10);
        assert_eq!(
            XGatewayCommon::trustee_sig_record(Chain::Bitcoin, bob()),
            10
        );

        assert_ok!(XGatewayCommon::move_trust_into_black_room(
            RawOrigin::Root.into(),
            Chain::Bitcoin,
            Some(vec![bob()]),
        ));

        assert_eq!(
            XGatewayCommon::little_black_house(Chain::Bitcoin),
            vec![bob()]
        );
        assert_eq!(XGatewayCommon::trustee_sig_record(Chain::Bitcoin, bob()), 0);

        assert!(XGatewayCommon::trustee_transition_status(Chain::Bitcoin));
    });
}

#[test]
fn test_claim_not_native_asset_reward() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(XGatewayCommon::do_trustee_election(Chain::Bitcoin), Ok(()));

        TrusteeSigRecord::<Test>::mutate(Chain::Bitcoin, bob(), |record| *record = 9);
        TrusteeSigRecord::<Test>::mutate(Chain::Bitcoin, charlie(), |record| *record = 1);

        assert_eq!(XGatewayCommon::trustee_sig_record(Chain::Bitcoin, bob()), 9);
        assert_eq!(
            XGatewayCommon::trustee_sig_record(Chain::Bitcoin, charlie()),
            1
        );
        assert_eq!(
            XGatewayCommon::trustee_sig_record(Chain::Bitcoin, dave()),
            0
        );

        let multi_address = XGatewayCommon::trustee_multisig_addr(Chain::Bitcoin).unwrap();

        assert_ok!(XGatewayRecords::deposit(&multi_address, X_BTC, 10));

        TrusteeSessionInfoOf::<Test>::mutate(Chain::Bitcoin, 1, |info| {
            if let Some(info) = info {
                info.0.trustee_list.iter_mut().for_each(|trustee| {
                    trustee.1 = XGatewayCommon::trustee_sig_record(Chain::Bitcoin, &trustee.0);
                });
            }
        });

        assert_ok!(XGatewayCommon::apply_claim_trustee_reward(1));

        assert_eq!(XAssets::usable_balance(&bob(), &X_BTC), 9);
        assert_eq!(XAssets::usable_balance(&charlie(), &X_BTC), 1);
    });
}
