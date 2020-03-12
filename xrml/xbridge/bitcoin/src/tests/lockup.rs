use super::*;

use crate::lockup::{detect_lockup_type, handle_lock_tx};
use crate::types::TxType;

#[test]
fn test_detect_tx_type() {
    with_externalities(&mut new_test_mainnet(), || {
        // 0.1 ~ 10 BTC
        XBridgeOfBTCLockup::set_locked_coin_limit((1 * 10000000, 10 * 100000000)).unwrap();

        // normal output addr 1FCaFxCdupMpxYKHpa83rUKGi1BygevJxF
        // opreturn ChainX:5UufyWdcgonrEHoqHb54DDUYJ4vmWg4NDBxWhrpizNcz2ptV@Polkadog:1FCa
        let notmal_output_0_1 = "0200000001336dbf5c2707d7dce56ae38d70ce189a95aa8a058771ac2b924dc657d781e90b010000006b483045022100a07fa7218ff1abb382af71b411464616787ddff068ead170bb8b1a7937f57f5d022028e1f2b498282b13ab8a3ad3f157a509d47e06229dd8a4ab97c8629debc97c100121034054cbf47712cb313eeba19d52941008fdf8460481049985221e0fc5a3e7e889ffffffff0280969800000000001976a9149bc21948187a4c40b5ef36a9266fa69fca5bd6a888ac0000000000000000476a45436861696e583a3555756679576463676f6e7245486f7148623534444455594a34766d5767344e44427857687270697a4e637a3270745640506f6c6b61646f673a3146436100000000".into();
        assert_eq!(detect_lockup_type::<Test>(&notmal_output_0_1), TxType::Lock);

        let notmal_output_10 = "0200000001336dbf5c2707d7dce56ae38d70ce189a95aa8a058771ac2b924dc657d781e90b010000006a47304402207e5d61039a5aae5b72aa5afcf19f08054aefea75be90298d342a642b7081687d022038db14e2caabd63db941b855d5aa2cada4395228ed272da1bcf147b57693e5a00121034054cbf47712cb313eeba19d52941008fdf8460481049985221e0fc5a3e7e889ffffffff0200ca9a3b000000001976a9149bc21948187a4c40b5ef36a9266fa69fca5bd6a888ac0000000000000000476a45436861696e583a3555756679576463676f6e7245486f7148623534444455594a34766d5767344e44427857687270697a4e637a3270745640506f6c6b61646f673a3146436100000000".into();
        assert_eq!(detect_lockup_type::<Test>(&notmal_output_10), TxType::Lock);

        // output is 900000
        let not_match_limit1 = "0200000001336dbf5c2707d7dce56ae38d70ce189a95aa8a058771ac2b924dc657d781e90b010000006a4730440220216cd4f96714b6f5615caaa5e54506280c8b200acd50d6ff933caad20dd1862802207ea23327e83fcaf68f74753ae8f421bbc7043d4808f0566bd03b84d57c163b740121034054cbf47712cb313eeba19d52941008fdf8460481049985221e0fc5a3e7e889ffffffff02a0bb0d00000000001976a9149bc21948187a4c40b5ef36a9266fa69fca5bd6a888ace803000000000000476a45436861696e583a3555756679576463676f6e7245486f7148623534444455594a34766d5767344e44427857687270697a4e637a3270745640506f6c6b61646f673a3146436100000000".into();
        assert_eq!(
            detect_lockup_type::<Test>(&not_match_limit1),
            TxType::Irrelevance
        );

        // output is 1000000001
        let not_match_limit2 = "0200000001336dbf5c2707d7dce56ae38d70ce189a95aa8a058771ac2b924dc657d781e90b010000006a473044022017b6c2c5fe70c1381790fc18a593543c67e95076e60ffe247978cdead3e1a69902205e49425b71291d3ac1d80a7c07d0fa97c9e97ff9363b0803bd132c7fe4ee849a0121034054cbf47712cb313eeba19d52941008fdf8460481049985221e0fc5a3e7e889ffffffff0201ca9a3b000000001976a9149bc21948187a4c40b5ef36a9266fa69fca5bd6a888ac0000000000000000476a45436861696e583a3555756679576463676f6e7245486f7148623534444455594a34766d5767344e44427857687270697a4e637a3270745640506f6c6b61646f673a3146436100000000".into();
        assert_eq!(
            detect_lockup_type::<Test>(&not_match_limit2),
            TxType::Irrelevance
        );

        // opreturn is ChainX:5UufyWdcgonrEHoqHb54DDUYJ4vmWg4NDBxWhrpizNcz2ptV:1abc
        let err_opreturn = "0200000001336dbf5c2707d7dce56ae38d70ce189a95aa8a058771ac2b924dc657d781e90b010000006a47304402202f21e5302cf3fbb8bb7894c481e0486902b7907d5dd3eb705a29e91450e1ddc9022070fea769f80597a7c069fe7bb9c8b52c3449bfce0bfc7f616234fd48667db60b0121034054cbf47712cb313eeba19d52941008fdf8460481049985221e0fc5a3e7e889ffffffff0200ca9a3b000000001976a9149bc21948187a4c40b5ef36a9266fa69fca5bd6a888ac00000000000000003e6a3c436861696e583a3555756679576463676f6e7245486f7148623534444455594a34766d5767344e44427857687270697a4e637a327074563a3161626300000000".into();
        assert_eq!(
            detect_lockup_type::<Test>(&err_opreturn),
            TxType::Irrelevance
        );
    })
}

#[test]
fn test_normal() {
    with_externalities(&mut new_test_mainnet(), || {
        use rstd::collections::btree_map::BTreeMap;
        // init
        // 0.001 ~ 10 BTC
        XBridgeOfBTCLockup::set_locked_coin_limit((1 * 100000, 10 * 100000000)).unwrap();
        // init assets
        let asset = xassets::Asset::new(
            XBridgeOfBTCLockup::TOKEN.to_vec(),
            b"Locked Bitcoin".to_vec(),
            xassets::Chain::Bitcoin,
            8,
            b"test".to_vec(),
        )
        .unwrap();
        assert_eq!(XAssets::register_asset(asset, true, true), Ok(()));
        let mut props: BTreeMap<xassets::AssetLimit, bool> = BTreeMap::new();
        props.insert(xassets::AssetLimit::CanMove, false);
        props.insert(xassets::AssetLimit::CanTransfer, false);
        props.insert(xassets::AssetLimit::CanWithdraw, false);
        props.insert(xassets::AssetLimit::CanDestroyWithdrawal, false);
        XAssets::set_asset_limit_props(XBridgeOfBTCLockup::TOKEN.to_vec(), props).unwrap();

        let public = hex!("fa6efb5db13089b4712305e39d0a16867c6822e3b1f4c4619937ae8a21961030")
            .unchecked_into();

        // 32ki -> 3CwG lock
        //      -> 32UN
        // hash 87ca6eca830427481781d42e746f9c85845acd11c671a340490805790770b255
        let fst_lock : Transaction= "0200000000010171aa0a9c43e308bb21952737aaabd52d13b8eba217848a962ad585ebda01229400000000171600141e6ad2476469e29df17d0b29779ec74992011ec4ffffffff03c0c62d000000000017a9147b5b8cabbfaf1a62cfc36414922db9caf8bb1bee87697906000000000017a91408938266a815783a42fbefb4402fe0e2f185e308870000000000000000466a44436861696e583a35564a504b3976706a667732486a766747645835367a755469365072377647765962794e354357455a79596162793777404c616f636975733a3343774702483045022100d38981cac1e51957d2f22c17d557c630851cee92e4982949ada6f50d356e303a02201d98713931aa95c114afb20deb6c355edc6970d67312019f019c4c8670428c64012103f165613dfa0ec1cca321423c11b10b8651ce51e4da8d463a7357cf5a5735261600000000".into();
        let fst_hash = fst_lock.hash();
        let addr1 =
            XBridgeOfBTC::verify_btc_address(b"3CwGi7JB9LMoiLfaUTbL9okXhiVbeDpygS").unwrap();
        // 32UN -> 35no lock
        //      -> 3QM8
        // hash a1c7905b782727edacfaa7e6ebec366628b48421a3169e10c106c463ed614369
        let snd_lock: Transaction = "0200000000010155b270077905084940a371c611cd5a84859c6f742ed4811748270483ca6eca870100000017160014669938a4883b41a59e62660f43a9aac3e283de8dffffffff03a06806000000000017a9142cf857756cfb1cd48e2fd750ff97e686ce5fe018874f0600000000000017a914f88847c7dd727db126bdcd1590f80c6867730a07870000000000000000466a44436861696e583a35564a504b3976706a667732486a766747645835367a755469365072377647765962794e354357455a79596162793777404c616f636975733a33356e6f02483045022100a36d5fd4262b7bedec8e7bd92115df71bfe2e1aa3590bfb2a7625b895881f79702205e7d49336018fca75ab2b7be8ae54414e2bd7c7a2234f8fb09d5c8909f67791d0121036ae9ef61cf64188f2b1830d4fd192719a9cf193a5f84eafca2aa10e0b6ffd50000000000".into();
        let snd_hash = snd_lock.hash();
        let addr2 =
            XBridgeOfBTC::verify_btc_address(b"35noA4eJhfzXdz6ruxChFAiGsbkRCpwQbi").unwrap();

        // 3CwG -> 32ki
        // 35no
        // hash 154fa53b363889c6cb8626dfb7121497aa0820d41d1c39fc3330959567820a73
        let trd_lock : Transaction= "0200000000010255b270077905084940a371c611cd5a84859c6f742ed4811748270483ca6eca870000000017160014c706e93e527a74f9b08dfb897e6667406832c6d2ffffffff694361ed63c406c1109e16a32184b4286636ecebe6a7faaced2727785b90c7a1000000001716001490ce8641913a41dee19796ea4fb54b0173971650ffffffff02261e34000000000017a9140bab8fa2ea965a0dc34b727c59b5dcfd91bc413f870000000000000000466a44436861696e583a35564a504b3976706a667732486a766747645835367a755469365072377647765962794e354357455a79596162793777404c616f636975733a33326b6902483045022100d36693c7c5019dfbe9ce2096167b1646813e7da55e031a48fcbec37b6bb529b0022013861a8c244f7a7fd750605bb6caa029a267a5ef95d629d3c1d07e613b5cb09101210201b23bcc02e8f1abc4e5860036234fd1b0e5d18e3e15d9e7f86dea9d7c4690ef02483045022100e9b42a33f69f43f7a5f1ce4b9aaa07bb2dfb2857ed23dede4c20304d2a81b91e02201b100aa94dae8520783589229dd8e2d8839aa317204fc2e98698d1f9db660135012103ae4939b53306061d9f075edee499c43393fece4632124c9a549cf67e44e20de400000000".into();
        let trd_hash = trd_lock.hash();
        let addr3 =
            XBridgeOfBTC::verify_btc_address(b"32kisvhbvSHWuZ76ivUiPN6dCGftMQZFpe").unwrap();

        // 32ki -> 32ki
        //      -> 3G1L
        // hash 400c1ba07048f92134c6306434f8e4fc8cf34e0b85a9b932059f728a6167953f
        let fth_lock: Transaction = "02000000000101730a826795953033fc391c1dd42008aa971412b7df2686cbc68938363ba54f1500000000171600141e6ad2476469e29df17d0b29779ec74992011ec4ffffffff0240420f000000000017a9140bab8fa2ea965a0dc34b727c59b5dcfd91bc413f87cfcd24000000000017a9149d0876aa518e9a21be823c598d179498cbcbbb6a8702473044022043db8986c6cb443f3d15bf0d358be87f5314ca241caaca5fd2fc6248ac38b93302206dedf4712d3536866679296aa3897de06bfeb5ad63f6d7dac7190a4ed1b6bc4d012103f165613dfa0ec1cca321423c11b10b8651ce51e4da8d463a7357cf5a5735261600000000".into();
        let fth_hash = fth_lock.hash();

        assert_eq!(detect_lockup_type::<Test>(&fst_lock), TxType::Lock);
        assert_eq!(detect_lockup_type::<Test>(&snd_lock), TxType::Lock);
        assert_eq!(detect_lockup_type::<Test>(&trd_lock), TxType::Lock);

        // handle first lock
        let r = handle_lock_tx::<Test>(&fst_lock, &fst_hash);
        assert_eq!(r, Ok(()));

        let value = XAssets::free_balance_of(&public, &XBridgeOfBTCLockup::TOKEN.to_vec());
        assert_eq!(value, 3000000);
        let r = XBridgeOfBTCLockup::address_locked_coin(&addr1);
        assert_eq!(r, 3000000);
        // check utxo
        assert_eq!(
            XBridgeOfBTCLockup::locked_up_btc((fst_hash, 0)).is_some(),
            true
        );

        // handle second lock
        let r = handle_lock_tx::<Test>(&snd_lock, &snd_hash);
        assert_eq!(r, Ok(()));

        let value = XAssets::free_balance_of(&public, &XBridgeOfBTCLockup::TOKEN.to_vec());
        assert_eq!(value, 3420000);
        let r = XBridgeOfBTCLockup::address_locked_coin(&addr2);
        assert_eq!(r, 420000);
        // check utxo
        assert_eq!(
            XBridgeOfBTCLockup::locked_up_btc((fst_hash, 0)).is_some(),
            true
        );
        assert_eq!(
            XBridgeOfBTCLockup::locked_up_btc((snd_hash, 0)).is_some(),
            true
        );

        // handle third lock
        let r = handle_lock_tx::<Test>(&trd_lock, &trd_hash);
        assert_eq!(r, Ok(()));

        let value = XAssets::free_balance_of(&public, &XBridgeOfBTCLockup::TOKEN.to_vec());
        assert_eq!(value, 3415590);
        let r = crate::lockup::AddressLockedCoin::<Test>::exists(&addr1);
        assert_eq!(r, false);
        let r = crate::lockup::AddressLockedCoin::<Test>::exists(&addr2);
        assert_eq!(r, false);
        let r = XBridgeOfBTCLockup::address_locked_coin(&addr3);
        assert_eq!(r, 3415590);
        // check utxo
        assert_eq!(
            XBridgeOfBTCLockup::locked_up_btc((fst_hash, 0)).is_some(),
            false
        );
        assert_eq!(
            XBridgeOfBTCLockup::locked_up_btc((snd_hash, 0)).is_some(),
            false
        );
        assert_eq!(
            XBridgeOfBTCLockup::locked_up_btc((trd_hash, 0)).is_some(),
            true
        );

        // unlock
        assert_eq!(detect_lockup_type::<Test>(&fth_lock), TxType::Unlock);
        crate::lockup::handle_unlock_tx::<Test>(&fth_lock, &fth_hash);
        let value = XAssets::free_balance_of(&public, &XBridgeOfBTCLockup::TOKEN.to_vec());
        assert_eq!(value, 0);
        let r = crate::lockup::AddressLockedCoin::<Test>::exists(&addr3);
        assert_eq!(r, false);
    })
}
