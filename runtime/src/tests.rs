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
}
