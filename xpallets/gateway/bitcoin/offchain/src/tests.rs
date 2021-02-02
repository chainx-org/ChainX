// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use light_bitcoin::chain::Transaction;
use light_bitcoin::keys::{Address, Network};
use light_bitcoin::script::Script;
use sp_core::crypto::{set_default_ss58_version, AccountId32, Ss58AddressFormat};
use sp_std::str::FromStr;
use xp_gateway_bitcoin::{
    extract_opreturn_data, AccountExtractor, BtcTxMetaType, BtcTxTypeDetector, OpReturnExtractor,
};
use xp_gateway_common::from_ss58_check;

#[test]
fn extract_account() {
    set_default_ss58_version(Ss58AddressFormat::SubstrateAccount);

    const DEPOSIT_HOT_ADDR: &str = "2N5QAjp4oaUbJCQqhsMiwSK1oYGJNUnAgqM";
    const DEPOSIT_COLD_ADDR: &str = "2N2AL9SfiGRssLt2bry6fnE4ruStLF7DtHD";
    let btc_tx_detector = BtcTxTypeDetector::new(
        Network::Testnet,
        0,
        (
            DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
            DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
        ),
        None,
    );

    let tx = "01000000021f3ffe48b4259a03792a48393028826fb8e5073bdaa68d9c3bd0b6c2c9bcad30010000006b483045022100b455f93c5b93a80255d4823ad5785b3aa6ab59cce03e045896b73f0f343ae1c702201de5d31272056168d81cf877313b033a5c47d643b38e6f36b4725f35929efa080121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d000000002a4d0feeb9c4a7901b3c0ebceb00f7fdfeba55a4528204deddbda9202bc0d08a010000006a47304402203b17fd6d2ceef10e56e7a28d46ce0937c88cdb8096f9807803199b7185159d4c02202c933afa661e333e5a648a1867b728bf63ee2ee91cc0117ad868ee1984703e710121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d0000000003605fa9000000000017a91485528a5e98cfb732129ff4a6b0d4d398c7be343687d4c91902000000001976a9147e836b50820d909dc10448ba7306a0f5dc6c755188ac0000000000000000326a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a434564425459444131447900000000".parse::<Transaction>().unwrap();
    let result = (
        Some((
            "5EWtScne4zWsGaP4gVo8DmLpChVx3MzoQTpKJCEdBTYDA1Dy"
                .parse::<AccountId32>()
                .unwrap(),
            None,
        )),
        11100000,
    );

    let (result, value) = result;
    let (op_return, deposit_value) =
        btc_tx_detector.parse_deposit_transaction_outputs(&tx, OpReturnExtractor::extract_account);
    assert_eq!(op_return, result);
    assert_eq!(deposit_value, value);

    for opreturn_script in tx
        .outputs
        .iter()
        .map(|output| Script::new(output.script_pubkey.clone()))
        .filter(|script| script.is_null_data_script())
    {
        assert_eq!(opreturn_script, Script::from_str("6a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a4345644254594441314479").unwrap());
        if let Some(info) = extract_opreturn_data(&opreturn_script)
            .and_then(|opreturn| OpReturnExtractor::extract_account(&opreturn))
        {
            let account_info = Some(info);
            assert_eq!(account_info, op_return);
            break;
        }
    }
    let script = Script::from_str("6a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a4345644254594441314479").unwrap();
    let data = extract_opreturn_data(&script).unwrap();
    let account = from_ss58_check(&data).unwrap();
    assert_eq!(
        account,
        "5EWtScne4zWsGaP4gVo8DmLpChVx3MzoQTpKJCEdBTYDA1Dy"
            .parse::<AccountId32>()
            .unwrap(),
    );
}

#[test]
fn detect_transaction_type() {
    set_default_ss58_version(Ss58AddressFormat::SubstrateAccount);

    const DEPOSIT_HOT_ADDR: &str = "2N5QAjp4oaUbJCQqhsMiwSK1oYGJNUnAgqM";
    const DEPOSIT_COLD_ADDR: &str = "2N2AL9SfiGRssLt2bry6fnE4ruStLF7DtHD";
    let btc_tx_detector = BtcTxTypeDetector::new(
        Network::Testnet,
        0,
        (
            DEPOSIT_HOT_ADDR.parse::<Address>().unwrap(),
            DEPOSIT_COLD_ADDR.parse::<Address>().unwrap(),
        ),
        None,
    );

    let withdrawal_tx = "0100000001f17b0338449c06fb1086a1295a146dcda8ae5283ff30a046b2858763fc604c6001000000fc0047304402201f4c3a0131e5217634cc092e173bc67e9c89d9ef4a13fe1e066e26a7490fe0ed02201a04bc72d039b243c121498ff7f75f246345d268e718a50e554828434b3e70a80147304402204b0f3057ca68222a53915d6c69cbf5a91335e058bfe0038db8ff288a98f791be02207a0fcc62a0c5218763a4ee0665c60a123a8c87107d19ca1531bbc99f35763bfa014c6952210376b9649206c74cc3dad6332c3a86d925a251bf9a55e6381f5d67b29a47559634210285eed6fa121c3a82ba6d0c37fa37e72bb06740761bfe9f294d2fa95fe237d5ba21036e1b175cc285b62a8b86e4ea94f32d627b36d60673b37eb3dd07d7b8c9ae6ddb53aeffffffff0220b38100000000001976a914b8a25f51dda9e7c856e705b4de3ae927caa8f35688acdef6c0030000000017a91485528a5e98cfb732129ff4a6b0d4d398c7be34368700000000".parse::<Transaction>().unwrap();
    let prev_withdrawal = "0100000001b3318003de3d26846eec1ae12857db4ec0370d3f4039723dac7e20fcf183855200000000fc0047304402203990765b1ef21404343cb01510a9c93ee8a16c472edb8eab2c31d0f4ffacf8e302202c396bd94c5738a4e1f35ca88aa436c08a843ad982e747e07edff1459e51adc30147304402206e07f7dcd8121f84e5c1310455c01f9416b297e60ca80955395dd182c0b2aa640220595edee7cfa0cdcd7b065ee589f8707f8d819e874e501047c815bfa21691ec45014c6952210376b9649206c74cc3dad6332c3a86d925a251bf9a55e6381f5d67b29a47559634210285eed6fa121c3a82ba6d0c37fa37e72bb06740761bfe9f294d2fa95fe237d5ba21036e1b175cc285b62a8b86e4ea94f32d627b36d60673b37eb3dd07d7b8c9ae6ddb53aeffffffff0220b38100000000001976a9147e836b50820d909dc10448ba7306a0f5dc6c755188ac6fd542040000000017a91485528a5e98cfb732129ff4a6b0d4d398c7be34368700000000".parse::<Transaction>().unwrap();
    let deposit_tx = "01000000031ab0ffbbbb1f0e4ff97455285f78a162863507eeabd691386108561e2367c21b010000006a473044022067530d34effce29e92fbc7e3b41a3dbc76b5b9c3eff8c06b21132e0813bc76e402206537d1a6c07d6616c73a741c31978c47b61eb1655ba7b84a4b40c37008485dbe0121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d0000000023f50fe9c93725527626b78f8ea3862c5bdd7dacd43a356b01532c537275d56a000000006b483045022100ebbfb39ea2492ed7170d8a6183c17eeb2513af1015ca13def4c4f5387731a40c02203ffe80dcfeda54e92e5302369a4697545bb9c04dfa432272b3860a54395d2a680121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d000000005d99c3a8a9187202982ef120c86ec4e6c99d6da9ea86a5dfb2dc920d5a774ecd010000006a47304402206e3b7d26ce1b62f49b28d30b7c9cc0fcb9be7655f570a861e6d2cb3cc5e3eb620220250c2ff7e4e90f8c39106a9db61c14caa103ecfe7c8cc3a3b338ffa9c099c78c0121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d0000000003002d31010000000017a91485528a5e98cfb732129ff4a6b0d4d398c7be34368760430700000000001976a9147e836b50820d909dc10448ba7306a0f5dc6c755188ac0000000000000000326a303545577453636e65347a57734761503467566f38446d4c7043685678334d7a6f5154704b4a434564425459444131447900000000".parse::<Transaction>().unwrap();
    let prev_deposit = "0100000003357a2fd556bd5f95a05748e1372ad2c1574a858dc5f2813f5112fd29539d91a8000000006a473044022045363e68aa48cc14be7c017b67385ab11b6785fd92608defab17c0215cbbd144022077938d87d788a783af09333e39273d60906ff82095074ee5ec5b5ca423641f3d0121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d0000000073620e68e3d29ec298512f0b5c9626ffaf23433e660205794eb53a99610b0752000000006a473044022076967123d6e8faab20f1ae8d6e8ef66f8255f549c2abc47bb56e68f62249a53802206653d23d1fa4967ca713ec1a9465d591bdef1c5d69029e150d07fdf0cd3754070121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d000000002d9b0716c1c317b86ee63589898b90656dd15a547c977c74824aab2c04f423e6000000006b483045022100f50117fe2ebb09455a832ee21f0a47bd24cff1147135d121352701dc923812bd02205bae8587fcc89facb020ef0db0c540b8b15d9c9aa5dd2410ba55a130d2bd6b290121032b42f71e3cb7be0f7f8bed0d8c8dd78a85141f268b9435dedb2eb1805b5f006d0000000003809698000000000017a91485528a5e98cfb732129ff4a6b0d4d398c7be34368760062600000000001976a9147e836b50820d909dc10448ba7306a0f5dc6c755188ac0000000000000000326a303547374c5a6538557463467335316471416e50537243385152374c53317175356a75364157373352715839346e6b314500000000".parse::<Transaction>().unwrap();

    assert_eq!(
        BtcTxMetaType::Withdrawal,
        btc_tx_detector.detect_transaction_type(
            &withdrawal_tx,
            Some(&prev_withdrawal),
            OpReturnExtractor::extract_account,
        )
    );
    // right : BtcTxMetaType::Deposit
    assert_ne!(
        BtcTxMetaType::Withdrawal,
        btc_tx_detector.detect_transaction_type(
            &deposit_tx,
            Some(&prev_deposit),
            OpReturnExtractor::extract_account,
        )
    )
}
