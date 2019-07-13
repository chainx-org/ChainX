// Copyright 2018-2019 Chainpool.

use primitives::traits::MaybeDebug;
use rstd::prelude::Vec;
use substrate_primitives::blake2::blake2_512;

use xr_primitives::{generic::b58, Name};
#[cfg(feature = "std")]
use xsupport::u8array_to_string;
use xsupport::{debug, error};

use crate::traits::Extractable;

/// Definition of something that the external world might want to say; its
/// existence implies that it has been checked and is good, particularly with
/// regards to the signature.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Extractor<AccountId>(Vec<u8>, ::rstd::marker::PhantomData<AccountId>);

fn split(data: &[u8]) -> Vec<Vec<u8>> {
    data.split(|x| *x == b'@').map(|d| d.to_vec()).collect()
}

const PREFIX: &[u8] = b"SS58PRE";

fn ss58hash(data: &[u8]) -> [u8; 64] {
    let mut v = PREFIX.to_vec();
    v.extend_from_slice(data);
    blake2_512(&v)
}

#[inline]
fn parse_chainx_addr<AccountId>(data: &[u8], addr_type: u8) -> Option<AccountId>
where
    AccountId: Default + MaybeDebug + AsMut<[u8]> + AsRef<[u8]>,
{
    let d: Vec<u8> = match b58::from(data) {
        Ok(a) => a,
        Err(_) => {
            error!(
                "[account_info]|base58 decode error|data:{:}",
                u8array_to_string(data)
            );
            return None;
        }
    };

    let mut res = AccountId::default();
    let len = res.as_ref().len();
    if d.len() != len + 3 {
        error!("[account_info]|Invalid length. AccountId is Public, Public is 32 bytes, 1 bytes version, 2 bytes checksum|need len:{:}|len:{:}",  len + 3, d.len());
        // Invalid length. AccountId is Public, Public is 32 bytes, 1 bytes version, 2 bytes checksum
        return None;
    }

    // Check if the deposit address matches the current network.
    // The first byte of pubkey indicates the network type.
    if d[0] != addr_type {
        error!(
            "[account_info]|data type error|need:{:}|current:{:}",
            addr_type, d[0]
        );
        return None;
    }
    // check checksum
    let hash = ss58hash(&d[0..len + 1]);
    if d[len + 1..len + 3] != hash[0..2] {
        error!(
            "[account_info]|ss58 checksum error|calc:{:?}|provide:{:?}",
            hash[0..2].to_vec(),
            d[len + 1..len + 3].to_vec()
        );
        return None;
    }
    // first byte is version
    res.as_mut().copy_from_slice(&d[1..len + 1]);
    Some(res)
}

pub fn parse_account_info<AccountId>(
    data: &[u8],
    addr_type: u8,
) -> Option<(AccountId, Option<Name>)>
where
    AccountId: Default + MaybeDebug + AsMut<[u8]> + AsRef<[u8]>,
{
    let v = split(data);
    if v.len() < 1 {
        error!(
            "[account_info]|can't parse data|data:{:}",
            u8array_to_string(data)
        );
        return None;
    }

    let op = &v[0];
    let res = if let Some(accountid) = parse_chainx_addr(op, addr_type) {
        accountid
    } else {
        return None;
    };

    // channel is a validator
    let channel_name = if v.len() > 1 {
        Some(v[1].to_vec())
    } else {
        None
    };

    debug!(
        "[account_info]|parse account info success!|who:{:?}|channel:{:?}",
        res, channel_name
    );
    Some((res, channel_name))
}

impl<AccountId> Extractable<AccountId> for Extractor<AccountId>
where
    AccountId: Default + MaybeDebug + AsMut<[u8]> + AsRef<[u8]>,
{
    /// same to `substrate/core/primitives/src/crypto.rs:trait Ss58Codec`
    fn account_info(data: &[u8], addr_type: u8) -> Option<(AccountId, Option<Name>)> {
        parse_account_info(data, addr_type)
    }
}

#[test]
fn test_extractor() {
    use rustc_hex::FromHex;
    use substrate_primitives::crypto::UncheckedInto;
    use substrate_primitives::H256;
    let addr: Vec<u8> = "f778a69d4166401048acb0f7b2625e9680609f8859c78e3d28e2549f84f0269a"
        .from_hex()
        .unwrap();
    let addr = H256::from_slice(&addr);

    {
        // test for ed25519 and channel
        use substrate_primitives::ed25519::Public;

        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
            44,
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));

        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4@channel1".as_bytes(),
            44,
        );
        assert_eq!(
            result,
            Some((addr.unchecked_into(), Some(b"channel1".to_vec())))
        );
    }
    {
        // test for sr25519
        use substrate_primitives::sr25519::Public;

        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
            44,
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
    {
        // test for checksum
        use substrate_primitives::ed25519::Public;
        // old checksum
        let addr: Vec<u8> = "00308187439ac204df9e299e1e54a00000000bf348e03dad679737c91871dc53"
            .from_hex()
            .unwrap();
        let addr = H256::from_slice(&addr);
        let result = Extractor::<Public>::account_info(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w2x5D".as_bytes(),
            42,
        );
        assert_eq!(result, None);
        // new checksum
        let result = Extractor::<Public>::account_info(
            "5C4xGQZwoNEM5mdk2U3vJbFZPr6ZKFSiqWnc9JRDcJ3w334p".as_bytes(),
            42,
        );
        assert_eq!(result, Some((addr.unchecked_into(), None)));
    }
    {
        // test for version
        use substrate_primitives::ed25519::Public;

        let result = Extractor::<Public>::account_info(
            "5VEW3R1T4LR3kDhYwXeeCnYrHRwRaH7E9V1KprypBe68XmY4".as_bytes(),
            42,
        );
        assert_eq!(result, None);
    }
}
