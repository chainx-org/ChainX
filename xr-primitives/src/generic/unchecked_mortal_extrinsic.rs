// Copyright 2019 Chainpool.

//! Generic implementation of an unchecked (pre-verification) extrinsic.

#[cfg(feature = "std")]
use std::fmt;

use parity_codec::{Decode, Encode, Input};

use rstd::prelude::*;
use runtime_io::blake2_256;
use runtime_primitives::{
    generic::Era,
    traits::{
        self, BlockNumberToHash, Checkable, CurrentHeight, Extrinsic, Lookup, MaybeDisplay, Member,
        SimpleArithmetic,
    },
};

use super::checked_extrinsic::CheckedExtrinsic;

const TRANSACTION_VERSION: u8 = 1;

/// A extrinsic right from the external world. This is unchecked and so
/// can contain a signature.
#[derive(PartialEq, Eq, Clone)]
pub struct UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration> {
    /// The signature, address, number of extrinsics have come before from
    /// the same signer and an era describing the longevity of this transaction,
    /// if this is a signed extrinsic.
    ///
    /// Acceleration More chances to be packed in block.
    pub signature: Option<(Address, Signature, Index, Era, Acceleration)>,
    /// The function that should be called.
    pub function: Call,
}

impl<Address, Index, Call, Signature, Acceleration>
    UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
{
    /// New instance of a signed extrinsic aka "transaction".
    pub fn new_signed(
        index: Index,
        function: Call,
        signed: Address,
        signature: Signature,
        era: Era,
        acceleration: Acceleration,
    ) -> Self {
        UncheckedMortalExtrinsic {
            signature: Some((signed, signature, index, era, acceleration)),
            function,
        }
    }

    /// New instance of an unsigned extrinsic aka "inherent".
    pub fn new_unsigned(function: Call) -> Self {
        UncheckedMortalExtrinsic {
            signature: None,
            function,
        }
    }
}

impl<Address: Encode, Index: Encode, Call: Encode, Signature: Encode, Acceleration: Encode>
    Extrinsic for UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
{
    fn is_signed(&self) -> Option<bool> {
        Some(self.signature.is_some())
    }
}

impl<Address, AccountId, Index, Call, Signature, Acceleration, Context, Hash, BlockNumber>
    Checkable<Context> for UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
where
    Address: Member + MaybeDisplay,
    Index: Encode + Member + MaybeDisplay + SimpleArithmetic,
    Call: Encode + Member,
    Signature: Member + traits::Verify<Signer = AccountId>,
    Acceleration: Encode + Member + MaybeDisplay + SimpleArithmetic + Copy,
    AccountId: Member + MaybeDisplay,
    BlockNumber: SimpleArithmetic,
    Hash: Encode,
    Context: Lookup<Source = Address, Target = AccountId>
        + CurrentHeight<BlockNumber = BlockNumber>
        + BlockNumberToHash<BlockNumber = BlockNumber, Hash = Hash>,
{
    type Checked = CheckedExtrinsic<AccountId, Index, Call, Acceleration>;

    fn check(self, context: &Context) -> Result<Self::Checked, &'static str> {
        Ok(match self.signature {
            Some((signed, signature, index, era, acceleration)) => {
                let h = context
                    .block_number_to_hash(BlockNumber::sa(
                        era.birth(context.current_height().as_()),
                    ))
                    .ok_or("transaction birth block ancient")?;
                let raw_payload = (index, self.function, era, h, acceleration);
                let signed = context.lookup(signed)?;
                if !raw_payload.using_encoded(|payload| {
                    if payload.len() > 256 {
                        signature.verify(&blake2_256(payload)[..], &signed)
                    } else {
                        signature.verify(payload, &signed)
                    }
                }) {
                    return Err("bad signature in extrinsic");
                }
                CheckedExtrinsic {
                    signed: Some((signed, raw_payload.0, raw_payload.4)),
                    function: raw_payload.1,
                }
            }
            None => CheckedExtrinsic {
                signed: None,
                function: self.function,
            },
        })
    }
}

impl<Address, Index, Call, Signature, Acceleration> Decode
    for UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
where
    Address: Decode,
    Signature: Decode,
    Index: Decode,
    Call: Decode,
    Acceleration: Decode,
{
    fn decode<I: Input>(input: &mut I) -> Option<Self> {
        // This is a little more complicated than usual since the binary format must be compatible
        // with substrate's generic `Vec<u8>` type. Basically this just means accepting that there
        // will be a prefix of vector length (we don't need
        // to use this).
        let _length_do_not_remove_me_see_above: Vec<()> = Decode::decode(input)?;

        let version = input.read_byte()?;

        let is_signed = version & 0b1000_0000 != 0;
        let version = version & 0b0111_1111;
        if version != TRANSACTION_VERSION {
            return None;
        }

        Some(UncheckedMortalExtrinsic {
            signature: if is_signed {
                Some(Decode::decode(input)?)
            } else {
                None
            },
            function: Decode::decode(input)?,
        })
    }
}

impl<Address, Index, Call, Signature, Acceleration> Encode
    for UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
where
    Address: Encode,
    Signature: Encode,
    Index: Encode,
    Call: Encode,
    Acceleration: Encode,
{
    fn encode(&self) -> Vec<u8> {
        super::encode_with_vec_prefix::<Self, _>(|v| {
            // 1 byte version id.
            match self.signature.as_ref() {
                Some(s) => {
                    v.push(TRANSACTION_VERSION | 0b1000_0000);
                    s.encode_to(v);
                }
                None => {
                    v.push(TRANSACTION_VERSION & 0b0111_1111);
                }
            }
            self.function.encode_to(v);
        })
    }
}

#[cfg(feature = "std")]
impl<Address: Encode, Index: Encode, Signature: Encode, Call: Encode, Acceleration: Encode>
    serde::Serialize for UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
{
    fn serialize<S>(&self, seq: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        self.using_encoded(|bytes| seq.serialize_bytes(bytes))
    }
}

/// TODO: use derive when possible.
#[cfg(feature = "std")]
impl<Address, Index, Call, Signature, Acceleration> fmt::Debug
    for UncheckedMortalExtrinsic<Address, Index, Call, Signature, Acceleration>
where
    Address: fmt::Debug,
    Index: fmt::Debug,
    Call: fmt::Debug,
    Acceleration: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UncheckedMortalExtrinsic({:?}, {:?})",
            self.signature.as_ref().map(|x| (&x.0, &x.2, &x.3, &x.4)),
            self.function
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_io::blake2_256;

    struct TestContext;
    impl Lookup for TestContext {
        type Source = u64;
        type Target = u64;
        fn lookup(&self, s: u64) -> Result<u64, &'static str> {
            Ok(s)
        }
    }
    impl CurrentHeight for TestContext {
        type BlockNumber = u64;
        fn current_height(&self) -> u64 {
            42
        }
    }
    impl BlockNumberToHash for TestContext {
        type BlockNumber = u64;
        type Hash = u64;
        fn block_number_to_hash(&self, n: u64) -> Option<u64> {
            Some(n)
        }
    }

    #[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Encode, Decode)]
    struct TestSig(u64, Vec<u8>);
    impl traits::Verify for TestSig {
        type Signer = u64;
        fn verify<L: traits::Lazy<[u8]>>(&self, mut msg: L, signer: &Self::Signer) -> bool {
            *signer == self.0 && msg.get() == &self.1[..]
        }
    }

    const DUMMY_ACCOUNTID: u64 = 0;

    type Ex = UncheckedMortalExtrinsic<u64, u64, Vec<u8>, TestSig, u32>;
    type CEx = CheckedExtrinsic<u64, u64, Vec<u8>, u32>;

    #[test]
    fn unsigned_codec_should_work() {
        let ux = Ex::new_unsigned(vec![0u8; 0]);
        let encoded = ux.encode();
        assert_eq!(Ex::decode(&mut &encoded[..]), Some(ux));
    }

    #[test]
    fn signed_codec_should_work() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(
                DUMMY_ACCOUNTID,
                (DUMMY_ACCOUNTID, vec![0u8; 0], Era::immortal(), 0u64, 1u32).encode(),
            ),
            Era::immortal(),
            1,
        );
        let encoded = ux.encode();
        assert_eq!(Ex::decode(&mut &encoded[..]), Some(ux));
    }

    #[test]
    fn unsigned_check_should_work() {
        let ux = Ex::new_unsigned(vec![0u8; 0]);
        assert!(!ux.is_signed().unwrap_or(false));
        assert!(<Ex as Checkable<TestContext>>::check(ux, &TestContext).is_ok());
    }

    #[test]
    fn badly_signed_check_should_fail() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(DUMMY_ACCOUNTID, vec![0u8]),
            Era::immortal(),
            1,
        );
        assert!(ux.is_signed().unwrap_or(false));
        assert_eq!(
            <Ex as Checkable<TestContext>>::check(ux, &TestContext),
            Err("bad signature in extrinsic")
        );
    }

    #[test]
    fn immortal_signed_check_should_work() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(
                DUMMY_ACCOUNTID,
                (DUMMY_ACCOUNTID, vec![0u8; 0], Era::immortal(), 0u64, 1u32).encode(),
            ),
            Era::immortal(),
            1,
        );
        assert!(ux.is_signed().unwrap_or(false));
        assert_eq!(
            <Ex as Checkable<TestContext>>::check(ux, &TestContext),
            Ok(CEx {
                signed: Some((DUMMY_ACCOUNTID, 0, 1)),
                function: vec![0u8; 0]
            })
        );
    }

    #[test]
    fn mortal_signed_check_should_work() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(
                DUMMY_ACCOUNTID,
                (
                    DUMMY_ACCOUNTID,
                    vec![0u8; 0],
                    Era::mortal(32, 42),
                    42u64,
                    1u32,
                )
                    .encode(),
            ),
            Era::mortal(32, 42),
            1,
        );
        assert!(ux.is_signed().unwrap_or(false));
        assert_eq!(
            <Ex as Checkable<TestContext>>::check(ux, &TestContext),
            Ok(CEx {
                signed: Some((DUMMY_ACCOUNTID, 0, 1)),
                function: vec![0u8; 0]
            })
        );
    }

    #[test]
    fn later_mortal_signed_check_should_work() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(
                DUMMY_ACCOUNTID,
                (
                    DUMMY_ACCOUNTID,
                    vec![0u8; 0],
                    Era::mortal(32, 11),
                    11u64,
                    1u32,
                )
                    .encode(),
            ),
            Era::mortal(32, 11),
            1,
        );
        assert!(ux.is_signed().unwrap_or(false));
        assert_eq!(
            <Ex as Checkable<TestContext>>::check(ux, &TestContext),
            Ok(CEx {
                signed: Some((DUMMY_ACCOUNTID, 0, 1)),
                function: vec![0u8; 0]
            })
        );
    }

    #[test]
    fn too_late_mortal_signed_check_should_fail() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(
                DUMMY_ACCOUNTID,
                (
                    DUMMY_ACCOUNTID,
                    vec![0u8; 0],
                    Era::mortal(32, 10),
                    10u64,
                    1u32,
                )
                    .encode(),
            ),
            Era::mortal(32, 10),
            1,
        );
        assert!(ux.is_signed().unwrap_or(false));
        assert_eq!(
            <Ex as Checkable<TestContext>>::check(ux, &TestContext),
            Err("bad signature in extrinsic")
        );
    }

    #[test]
    fn too_early_mortal_signed_check_should_fail() {
        let ux = Ex::new_signed(
            0,
            vec![0u8; 0],
            DUMMY_ACCOUNTID,
            TestSig(
                DUMMY_ACCOUNTID,
                (
                    DUMMY_ACCOUNTID,
                    vec![0u8; 0],
                    Era::mortal(32, 43),
                    43u64,
                    1u32,
                )
                    .encode(),
            ),
            Era::mortal(32, 43),
            1,
        );
        assert!(ux.is_signed().unwrap_or(false));
        assert_eq!(
            <Ex as Checkable<TestContext>>::check(ux, &TestContext),
            Err("bad signature in extrinsic")
        );
    }

    #[test]
    fn encoding_matches_vec() {
        let ex = Ex::new_unsigned(vec![0u8; 0]);
        let encoded = ex.encode();
        let decoded = Ex::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded, ex);
        let as_vec: Vec<u8> = Decode::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(as_vec.encode(), encoded);
    }
}
