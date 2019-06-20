#![cfg(test)]

use super::*;
use hex_literal::hex;
#[test]
fn check_header_with_same() {
    use substrate_primitives::ed25519::Public;
    use xfisher::CheckHeader;

    let pubkey: AccountId = Public(hex!(
        "dc6731942968ba7a321b22f688fbb94449975b5b078f2a495610df52c0fa2fbf"
    ));
    let header: xfisher::RawHeader = hex!("d40fc01c49faf8904392016f26ad78d1a278f0f24cd23f86ef515effc0c2a1a62901109f11d0d043c906d7f900327b0e997a9c87e92f039063a76cbb5c4d76657ca721be011fe0bfad5ac066befbf918750a3705524c0b96ca5d01d4784856737adf00").to_vec();
    let sig: H512 = hex!("420684ff8a126699398146647fb9b484e629419cd23a62f40d9d8e410b1aef29f11d364b32a339fb1bc78a9d0a745a6c2759678ee5e273c01e2648c906ea6204").into();
    let slot: u64 = 780332910;

    let header = (header, slot, sig);

    // normal check
    let r = HeaderChecker::check_header(&pubkey, &header, &header.clone());
    assert_eq!(r, Err("same header, do nothing for this"));

    // all header should failed
    let all_header: xfisher::RawHeader = hex!("38cc10775c83d2effcfaca9716c83c45dbbe679e8c19bbf722dae6fbcaca7859190164bc5bdfcca7a9eff2c747d484a1c003d0148199e5559081c6f8c407bcdc47db5e9286d974a659b7a0dac52a540b744eed55712ebb0f83d1ce67594a51e815cd040461757261210168ef822e00000000aab2d3ad598426568deb407cd871db7f731a404949ba5a155b05d3cd50e07338e353f62d198a348ea752e6d96d987f1337830232a8011319e3f93ee2e040fc04").to_vec();
    let all_header = (all_header, slot, sig);
    let r = HeaderChecker::check_header(&pubkey, &header, &all_header);
    assert_eq!(r, Err("should use pre header"));

    // normal check2
    let pubkey: AccountId = Public(hex!(
        "720aaf07da947bdf7ac25f73cc70207b1e691f2794ee2044fdd0442bda78ed31"
    ));

    let header: xfisher::RawHeader = hex!("b9f82b092166d6ec8c6c7fa8ec87e035f9c54599eb850c2e19151c179f28cfad2964998b7679c533be2d84de0c4038188bdec3f4f1640e6034a33b474657033156eccd1b36c5dc9a5ef72ede62162f5ff0e74f82818bab2d5dd3c7e7b12e4375a90200").to_vec();
    let slot: u64 = 780431957;
    let sig: H512 = hex!("b6e87db4a951aa2d52af38b1b52b5d2c2024e9df277ab30ad38ce4c7abc84c344fea0f79ed22db31aacffd5367a5f374313d553cc69d596fc49360620fbedb01").into();

    let header1 = (header, slot, sig);

    let header: xfisher::RawHeader = hex!("15007dd167077811727b6777f056b19b65083ed27685d6dc0cc904de46f1ad913d69e3590805d5a295db893ff1e26dab8f8c937cbc5bbb826f94d51003c10ba18f62cd1b36c5dc9a5ef72ede62162f5ff0e74f82818bab2d5dd3c7e7b12e4375a90200").to_vec();
    let slot: u64 = 780431957;
    let sig: H512 = hex!("18554f60f8dd44bd55019d0be45c011118cc02bec734a87dd47b9015a1b8aef63c4bed2ad874f17a7528688b8d74cebceebefd08ff1ea64d95a65f26a938de0d").into();
    let header2 = (header, slot, sig);

    let r = HeaderChecker::check_header(&pubkey, &header1, &header2);
    assert_eq!(r, Ok((6410, 6735)));
}
