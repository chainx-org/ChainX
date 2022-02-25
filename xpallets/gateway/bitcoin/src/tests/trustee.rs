// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::assert_noop;
use hex_literal::hex;

use light_bitcoin::{
    chain::Transaction,
    crypto::dhash160,
    keys::{Address, AddressTypes, Network, Public, Type},
    mast::Mast,
    script::{Builder, Opcode},
    serialization::{self, Reader},
};

use xpallet_gateway_common::traits::TrusteeForChain;

use crate::mock::{
    ExtBuilder, Test, XGatewayBitcoin, XGatewayBitcoinErr
};
use crate::{
    trustee::create_multi_address,
    tx::validator::parse_and_check_signed_tx_impl
};
use sp_std::convert::TryInto;

#[test]
pub fn test_check_trustee_entity() {
    ExtBuilder::default().build_and_execute(|| {
        let addr_ok_3 = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
        let public3 = Public::from_slice(&addr_ok_3).unwrap();
        assert_eq!(XGatewayBitcoin::check_trustee_entity(&addr_ok_3).unwrap().0, public3);
        let addr_ok_2 = hex!("0211252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
        let public2 = Public::from_slice(&addr_ok_2).unwrap();
        assert_eq!(XGatewayBitcoin::check_trustee_entity(&addr_ok_2).unwrap().0, public2);

        let addr_too_long = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40cc");
        assert_noop!(XGatewayBitcoin::check_trustee_entity(&addr_too_long), XGatewayBitcoinErr::InvalidPublicKey);
        let addr_normal = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae4011252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
        assert_noop!(XGatewayBitcoin::check_trustee_entity(&addr_normal), XGatewayBitcoinErr::InvalidPublicKey);
        let addr_err_type = hex!("0411252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
        assert_noop!(XGatewayBitcoin::check_trustee_entity(&addr_err_type), XGatewayBitcoinErr::InvalidPublicKey);
        let addr_zero = hex!("020000000000000000000000000000000000000000000000000000000000000000");
        assert_noop!(XGatewayBitcoin::check_trustee_entity(&addr_zero), XGatewayBitcoinErr::InvalidPublicKey);
        let addr_ec_p = hex!("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f");
        assert_noop!(XGatewayBitcoin::check_trustee_entity(&addr_ec_p), XGatewayBitcoinErr::InvalidPublicKey);
        let addr_ec_p_2 = hex!("02fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc3f");
        assert_noop!(XGatewayBitcoin::check_trustee_entity(&addr_ec_p_2), XGatewayBitcoinErr::InvalidPublicKey);
    })
}

#[test]
pub fn test_multi_address() {
    let pubkey1_bytes = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let pubkey2_bytes = hex!("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2");
    let pubkey3_bytes = hex!("023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d");

    let script = Builder::default()
        .push_opcode(Opcode::OP_2)
        .push_bytes(&pubkey1_bytes)
        .push_bytes(&pubkey2_bytes)
        .push_bytes(&pubkey3_bytes)
        .push_opcode(Opcode::OP_3)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();
    let multisig_address = Address {
        kind: Type::P2SH,
        network: Network::Testnet,
        hash: AddressTypes::Legacy(dhash160(&script)),
    };
    assert_eq!(
        "2MtAUgQmdobnz2mu8zRXGSTwUv9csWcNwLU",
        multisig_address.to_string()
    );
}

#[test]
fn test_create_multi_address() {
    let mut hot_keys = Vec::new();
    let pubkey1_bytes = hex!("03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba");
    let pubkey2_bytes = hex!("0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd");
    let pubkey3_bytes = hex!("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let pubkey4_bytes = hex!("0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3");
    hot_keys.push(Public::from_slice(&pubkey1_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey2_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey3_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey4_bytes).unwrap());

    let mut cold_keys = Vec::new();
    let pubkey5_bytes = hex!("02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee");
    let pubkey6_bytes = hex!("03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780");
    let pubkey7_bytes = hex!("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2");
    let pubkey8_bytes = hex!("020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f");
    cold_keys.push(Public::from_slice(&pubkey5_bytes).unwrap());
    cold_keys.push(Public::from_slice(&pubkey6_bytes).unwrap());
    cold_keys.push(Public::from_slice(&pubkey7_bytes).unwrap());
    cold_keys.push(Public::from_slice(&pubkey8_bytes).unwrap());

    ExtBuilder::default().build_and_execute(|| {
        let hot_info = create_multi_address::<Test>(&hot_keys, 3).unwrap();
        let cold_info = create_multi_address::<Test>(&cold_keys, 3).unwrap();
        let real_hot_addr = b"2N1CPZyyoKj1wFz2Fy4gEHpSCVxx44GtyoY".to_vec();
        let real_cold_addr = b"2N24ytjE3MtkMpYWo8LrTfnkbpyaJGyQbCA".to_vec();
        assert_eq!(hot_info.addr, real_hot_addr);
        assert_eq!(cold_info.addr, real_cold_addr);

        let pks = [
            169, 20, 87, 55, 193, 151, 147, 67, 146, 12, 238, 164, 14, 124, 125, 104, 178, 100,
            176, 239, 250, 62, 135,
        ];
        let addr: Address = String::from_utf8_lossy(&hot_info.addr).parse().unwrap();
        let pk = match addr.hash {
            AddressTypes::Legacy(h) => h.as_bytes().to_vec(),
            AddressTypes::WitnessV0ScriptHash(_) => todo!(),
            AddressTypes::WitnessV0KeyHash(_) => todo!(),
            AddressTypes::WitnessV1Taproot(_) => todo!(),
        };
        let mut pubkeys = Vec::new();
        pubkeys.push(Opcode::OP_HASH160 as u8);
        pubkeys.push(Opcode::OP_PUSHBYTES_20 as u8);
        pubkeys.extend_from_slice(&pk);
        pubkeys.push(Opcode::OP_EQUAL as u8);
        assert_eq!(pubkeys, pks);
    });
}

#[test]
fn test_verify_signed() {
    let full_sig_tx = "010000000317840b38d466580696e9cb065c7a7aa55cb58cd5eb2526a10c3a30cc06d4b50a05000000fdfd0000483045022100dabbf878df8cacb23c08a8b5414cd64392a3f84777db4c01d8eec1e06d2e03fb0220502bd6e3960b68452699a40debfd92ac02e45d1526a2b570f5b28abdb496706401473044022047c58c3ad586d93f4b4caf65230a21e0ff70475b66affb8d4f92e916e6f6f664022029231b30472a949648dd99585ccbb169ccc2c007ad5387f580d41affdc8b37b6014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff853c87b1ecb4e881f323fec5314cb8623ca15de1341694e8352f99c434e7046a02000000fdfe0000483045022100b1b2233f70434f4079c1a8be1be5843b4dfe1edea30a3533aa94781af9984b2e02201ef78527ced51c7b122568666b9499d9cd2d4c3e704f5a54ebe433489c91b20101483045022100bde660b2f6f3c6fa512794377564289cbfcbeab6ecba1fe3b0b1531ebaa7d00a02207ea5435312280e0b502de715a6cbff7de866ba508a5fe8a644b88540ed471aee014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff442214a2d5a31195d6849005699892f60d48d89bca15bdb4ad6349c083e9936202000000fdfd000047304402205960c277575a7d2bb719211fe9cee0dd398c5a64d3a258fb0f877ae176dd11af02206cc0be53b1d5ea59477f9d2103ce06b61608561ac466c72235e86b26fe45734d01483045022100dcbd79d6f2d9504e2ea1578b7fdc9f98dadc018708acb4b87bd8b154312edfaa022043197a5b72219dc9603a81146a65c724a09022229ada2e3101a002dbd834b591014c69522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253aeffffffff0340ebd201000000001976a9148e2fbed4fc7481a9a51f2bfe204301a122473f2f88ac406fdf25000000001976a914ede61104eddc07594f0c0cf43fecb9675353d16288ac91a3f6070000000017a914cb94110435d0635223eebe25ed2aaabc03781c458700000000".parse().unwrap();
    let script = "522102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d2103a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad10253ae".parse().unwrap();
    ExtBuilder::default().build_and_execute(|| {
        let result = parse_and_check_signed_tx_impl::<Test>(&full_sig_tx, script);
        assert_eq!(result, Ok(2))
    });
}

#[test]
fn test_verify_tx_sign() {
    ExtBuilder::default().build_and_execute(|| {
        let script_hex = "542102e2b2720a9e54617ba87fca287c3d7f9124154d30fa8dc9cd260b6b254e1d7aea210219fc860933a1362bc5e0a0bbe1b33a47aedf904765f4a85cd166ba1d767927ee2102b921cb319a14c6887b12cee457453f720e88808a735a578d6c57aba0c74e5af32102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210346aa7ade0b567b34182cacf9444deb44ee829e14705dc87175107dd09d5dbf4021034d3e7f87e69c6c71df6052b44f9ed99a3d811613140ebf09f8fdaf904a2e1de856ae";
        let cases = vec![
            (
                "0100000001abbd850cf083bbfa367081718c7efd911e56ffd849ae48e812c861adf253ef6101000000fdf5010047304402206f926dc8324f20321114353a48c5b8cc64dd5b7a97a33f9dee4f4aa92a2c80cb02203402cc039f47e557dd97a365f7395bd83427291dcc9e898f029ad18e5f9f15d001483045022100b83283df05ac293ba1996a4bab5bbc7f07e874e4209ada57696c91109908f6c20220026d9dd170a01ae2ab5baec934c1a6c073fe1340b24cb27456d644947a74387d01483045022100c380b2955f90e0a1a5762753237661c25d55039a711ed90d8424642c7c3c978b02204a3c2d817040f3f332aa75edeb7ce778d909f05a86ebe27f57776113362459860148304502210081d668bf752424c89e208cf9789e7449c080a2cd9fda6a518ac36d81e5d760dd02206353eaad7e587602ab6665a5788a2c831e9d08bf8685dd250370dbd978a54665014ccf542102e2b2720a9e54617ba87fca287c3d7f9124154d30fa8dc9cd260b6b254e1d7aea210219fc860933a1362bc5e0a0bbe1b33a47aedf904765f4a85cd166ba1d767927ee2102b921cb319a14c6887b12cee457453f720e88808a735a578d6c57aba0c74e5af32102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210346aa7ade0b567b34182cacf9444deb44ee829e14705dc87175107dd09d5dbf4021034d3e7f87e69c6c71df6052b44f9ed99a3d811613140ebf09f8fdaf904a2e1de856aeffffffff03d2622000000000001976a914b9944df543bc909b527351311c5a01a78a3271e788acff40330e0000000017a9149079c3650e5a9799afa552cbbcc280e45d52117c8777d778e50000000017a914d246f700f4969106291a75ba85ad863cae68d6678700000000",
                4
            ),
            (
                "0100000001abbd850cf083bbfa367081718c7efd911e56ffd849ae48e812c861adf253ef6101000000fdac010047304402206f926dc8324f20321114353a48c5b8cc64dd5b7a97a33f9dee4f4aa92a2c80cb02203402cc039f47e557dd97a365f7395bd83427291dcc9e898f029ad18e5f9f15d001483045022100b83283df05ac293ba1996a4bab5bbc7f07e874e4209ada57696c91109908f6c20220026d9dd170a01ae2ab5baec934c1a6c073fe1340b24cb27456d644947a74387d0148304502210081d668bf752424c89e208cf9789e7449c080a2cd9fda6a518ac36d81e5d760dd02206353eaad7e587602ab6665a5788a2c831e9d08bf8685dd250370dbd978a54665014ccf542102e2b2720a9e54617ba87fca287c3d7f9124154d30fa8dc9cd260b6b254e1d7aea210219fc860933a1362bc5e0a0bbe1b33a47aedf904765f4a85cd166ba1d767927ee2102b921cb319a14c6887b12cee457453f720e88808a735a578d6c57aba0c74e5af32102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210346aa7ade0b567b34182cacf9444deb44ee829e14705dc87175107dd09d5dbf4021034d3e7f87e69c6c71df6052b44f9ed99a3d811613140ebf09f8fdaf904a2e1de856aeffffffff03d2622000000000001976a914b9944df543bc909b527351311c5a01a78a3271e788acff40330e0000000017a9149079c3650e5a9799afa552cbbcc280e45d52117c8777d778e50000000017a914d246f700f4969106291a75ba85ad863cae68d6678700000000",
                3
            ),
            (
                "0100000001abbd850cf083bbfa367081718c7efd911e56ffd849ae48e812c861adf253ef6101000000fd63010047304402206f926dc8324f20321114353a48c5b8cc64dd5b7a97a33f9dee4f4aa92a2c80cb02203402cc039f47e557dd97a365f7395bd83427291dcc9e898f029ad18e5f9f15d00148304502210081d668bf752424c89e208cf9789e7449c080a2cd9fda6a518ac36d81e5d760dd02206353eaad7e587602ab6665a5788a2c831e9d08bf8685dd250370dbd978a54665014ccf542102e2b2720a9e54617ba87fca287c3d7f9124154d30fa8dc9cd260b6b254e1d7aea210219fc860933a1362bc5e0a0bbe1b33a47aedf904765f4a85cd166ba1d767927ee2102b921cb319a14c6887b12cee457453f720e88808a735a578d6c57aba0c74e5af32102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210346aa7ade0b567b34182cacf9444deb44ee829e14705dc87175107dd09d5dbf4021034d3e7f87e69c6c71df6052b44f9ed99a3d811613140ebf09f8fdaf904a2e1de856aeffffffff03d2622000000000001976a914b9944df543bc909b527351311c5a01a78a3271e788acff40330e0000000017a9149079c3650e5a9799afa552cbbcc280e45d52117c8777d778e50000000017a914d246f700f4969106291a75ba85ad863cae68d6678700000000",
                2
            ),
            (
                "0100000001abbd850cf083bbfa367081718c7efd911e56ffd849ae48e812c861adf253ef6101000000fd1b010048304502210081d668bf752424c89e208cf9789e7449c080a2cd9fda6a518ac36d81e5d760dd02206353eaad7e587602ab6665a5788a2c831e9d08bf8685dd250370dbd978a54665014ccf542102e2b2720a9e54617ba87fca287c3d7f9124154d30fa8dc9cd260b6b254e1d7aea210219fc860933a1362bc5e0a0bbe1b33a47aedf904765f4a85cd166ba1d767927ee2102b921cb319a14c6887b12cee457453f720e88808a735a578d6c57aba0c74e5af32102df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6210346aa7ade0b567b34182cacf9444deb44ee829e14705dc87175107dd09d5dbf4021034d3e7f87e69c6c71df6052b44f9ed99a3d811613140ebf09f8fdaf904a2e1de856aeffffffff03d2622000000000001976a914b9944df543bc909b527351311c5a01a78a3271e788acff40330e0000000017a9149079c3650e5a9799afa552cbbcc280e45d52117c8777d778e50000000017a914d246f700f4969106291a75ba85ad863cae68d6678700000000",
                1
            )
        ];

        for (tx_hex, expect) in cases {
            let bytes = hex::decode(tx_hex).unwrap();
            let tx: Transaction = serialization::deserialize(Reader::new(&bytes)).unwrap();
            let script = script_hex.parse().unwrap();
            let got = parse_and_check_signed_tx_impl::<Test>(&tx, script);
            assert_eq!(got, Ok(expect));
        }
    });
}

#[test]
fn test_create_taproot_address() {
    let mut hot_keys = Vec::new();
    let pubkey1_bytes = hex!("0283f579dd2380bd31355d066086e1b4d46b518987c1f8a64d4c0101560280eae2");
    let pubkey2_bytes = hex!("027a0868a14bd18e2e45ff3ad960f892df8d0edd1a5685f0a1dc63c7986d4ad55d");
    let pubkey3_bytes = hex!("02c9929543dfa1e0bb84891acd47bfa6546b05e26b7a04af8eb6765fcc969d565f");
    hot_keys.push(Public::from_slice(&pubkey1_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey2_bytes).unwrap());
    hot_keys.push(Public::from_slice(&pubkey3_bytes).unwrap());
    ExtBuilder::default().build_and_execute(|| {
        let pks = hot_keys
            .into_iter()
            .map(|k| k.try_into().unwrap())
            .collect::<Vec<_>>();
        let threshold_addr: Address = Mast::new(pks, 2 as usize)
            .unwrap()
            .generate_address(&crate::Pallet::<Test>::network_id().to_string())
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            threshold_addr.to_string(),
            "tb1pn202yeugfa25nssxk2hv902kmxrnp7g9xt487u256n20jgahuwasdcjfdw"
        )
    })
}
