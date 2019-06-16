// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use ethereum_types::H160;
use hex_literal::hex;

use super::*;

#[test]
fn test_recover_eth_address() {
    // ethereum tx hash = 0xb81680f5224bdd2e7b55075f1332af1e85026b448ff38d3ae2e8e014188ddec0
    // chainx extrinsic hash = 0x67590e9ae9f34c35e6743602c64e17f75b92c261f2740b88593c8700c05ac554
    let raw = hex!("f8571d8501dcd6500082ae809482e2b7d189a81a251eaa51ac31871f8c4b91dff480b635536a4a6846514b5456544c62646359675635655a45687853436f347171594e4d47354c7242616e4157434731665a6740446556616c");
    let data = hex!("35536a4a6846514b5456544c62646359675635655a45687853436f347171594e4d47354c7242616e4157434731665a6740446556616c");
    assert!(contains(&raw, &data));

    let signature = EcdsaSignature(
        [
            // hex: 2105f2b5b8476d4b4494da61565b053b5b59d50d5b8162cd67f0617ecf30ff86
            33, 5, 242, 181, 184, 71, 109, 75, 68, 148, 218, 97, 86, 91, 5, 59, 91, 89, 213, 13, 91,
            129, 98, 205, 103, 240, 97, 126, 207, 48, 255, 134,
        ],
        [
            // hex: 7d34deb9e0396f1addee0ac1ccde1e50c6bcacd34e667efd832f05ccdc4a92de
            125, 52, 222, 185, 224, 57, 111, 26, 221, 238, 10, 193, 204, 222, 30, 80, 198, 188, 172,
            211, 78, 102, 126, 253, 131, 47, 5, 204, 220, 74, 146, 222,
        ],
        0, // original v = 0x1b (27), standard v = 0x00 (0),
    );
    let from = H160::from_slice(&hex!("82e2b7d189a81a251eaa51ac31871f8c4b91dff4"));
    assert_eq!(eth_recover(&signature, &raw), Some(from.to_fixed_bytes()));

    assert_eq!(
        recover_eth_address(&signature, &raw, &data),
        Some(from.to_fixed_bytes())
    );

    // ethereum tx hash = 0x50d50ba8ff02c924998422b61b768aec959f1f04e7dfe9b861f2801fab444e9a
    // chainx extrinsic hash = 0xfd375aaa87b95f029484f00918cce192eb6a459bea06f32b6fe9c3339ad4944c
    let raw = hex!("f8561b84b2d05e0082ae809482e2b7d189a81a251eaa51ac31871f8c4b91dff480b635513158394e50473537715a504c416658647244376b4779573759356b4448456b51574a374146457a785a544b466b5440446556616c");
    let data = hex!("35513158394e50473537715a504c416658647244376b4779573759356b4448456b51574a374146457a785a544b466b5440446556616c");
    assert!(contains(&raw, &data));

    let signature = EcdsaSignature(
        [
            // hex: ea2ebb221a3e29e93b035fba841f351b4dc32a76a5690dba501ee22620e1a1f3
            234, 46, 187, 34, 26, 62, 41, 233, 59, 3, 95, 186, 132, 31, 53, 27, 77, 195, 42, 118,
            165, 105, 13, 186, 80, 30, 226, 38, 32, 225, 161, 243,
        ],
        [
            // hex: 1b59bbc33241acadf44626205a5084e3fdb792a93cecfaec7fa0181b99516378
            27, 89, 187, 195, 50, 65, 172, 173, 244, 70, 38, 32, 90, 80, 132, 227, 253, 183, 146,
            169, 60, 236, 250, 236, 127, 160, 24, 27, 153, 81, 99, 120,
        ],
        1, // original v = 0x1c (28), standard v = 0x01 (1),
    );
    let from = H160::from_slice(&hex!("82e2b7d189a81a251eaa51ac31871f8c4b91dff4"));
    assert_eq!(eth_recover(&signature, &raw), Some(from.to_fixed_bytes()));

    assert_eq!(
        recover_eth_address(&signature, &raw, &data),
        Some(from.to_fixed_bytes())
    );

    // ethereum tx hash = 0x5859e92c71dc852d4eb14e6761468bf7af2e8747a8946e27714accd4c068e69e
    // chainx extrinsic hash = 0x7461295e2ae8f43b30a9be56a01abbbaf6d6d2399725c3f79709c1c24b6e1a39
    let raw = hex!("f85331843b9aca0082c35094a72ab17504430a49c185921d9471fe7d0199e47680b035506e7556545744736f45653855384b4c37324d4c6b70663559456d64374241647445387258376b62586b5339544836018080");
    let data = hex!("35506e7556545744736f45653855384b4c37324d4c6b70663559456d64374241647445387258376b62586b5339544836");
    assert!(contains(&raw, &data));

    let signature = EcdsaSignature(
        [
            // hex: c80d43fb687bc67291a7584b7c00a77875d5fe345dddf62eb9bbc552e776a19f
            200, 13, 67, 251, 104, 123, 198, 114, 145, 167, 88, 75, 124, 0, 167, 120, 117, 213, 254,
            52, 93, 221, 246, 46, 185, 187, 197, 82, 231, 118, 161, 159,
        ],
        [
            // hex: 540a8a1d8a205c83dec19da30be305e08da7de757f86ac684d2a11bac9445b44
            84, 10, 138, 29, 138, 32, 92, 131, 222, 193, 157, 163, 11, 227, 5, 224, 141, 167, 222,
            117, 127, 134, 172, 104, 77, 42, 17, 186, 201, 68, 91, 68,
        ],
        0, // original v = 0x25 (37), standard v = 0x00 (0),
    );
    let from = H160::from_slice(&hex!("ec27421edc22ae46c23ad1e8b34f8651b3d1d350"));
    assert_eq!(eth_recover(&signature, &raw), Some(from.to_fixed_bytes()));

    assert_eq!(
        recover_eth_address(&signature, &raw, &data),
        Some(from.to_fixed_bytes())
    );

    // ethereum tx hash = 0x820a551477850a692144fb2c866675ee6623273afb26e019e97837f18ac305fe
    // chainx extrinsic hash = 0x71d1d8782518887dbccb6b86c3447fa050ff14b816fc29ced6199886d89076f8
    let raw = hex!("f8540e850227f0630082c3509448c2ea333b6ac99d6486cbd4a3ddee637a02536780b035516d77777861436a794874324b506b43775173414241507639704b46374e7a463957485a566a3648317a635a705765018080");
    let data = hex!("35516d77777861436a794874324b506b43775173414241507639704b46374e7a463957485a566a3648317a635a705765");
    assert!(contains(&raw, &data));

    let signature = EcdsaSignature(
        [
            // hex: 94346bb30562e4bb0ff6b3ca4d7125d9f4e957ac8449cec19fb3af7bfd129ce4
            148, 52, 107, 179, 5, 98, 228, 187, 15, 246, 179, 202, 77, 113, 37, 217, 244, 233, 87,
            172, 132, 73, 206, 193, 159, 179, 175, 123, 253, 18, 156, 228,
        ],
        [
            // hex: 4871d783b4b31b71ec15d310ebea5ebea41de27324b6837584b1c67a77799843
            72, 113, 215, 131, 180, 179, 27, 113, 236, 21, 211, 16, 235, 234, 94, 190, 164, 29, 226,
            115, 36, 182, 131, 117, 132, 177, 198, 122, 119, 121, 152, 67,
        ],
        1, // original v = 0x26 (38), standard v = 0x01 (1),
    );
    let from = H160::from_slice(&hex!("f77fd2297cb28b7a104f3f4d47b19a50a1ddd451"));
    assert_eq!(eth_recover(&signature, &raw), Some(from.to_fixed_bytes()));

    assert_eq!(
        recover_eth_address(&signature, &raw, &data),
        Some(from.to_fixed_bytes())
    );
}
