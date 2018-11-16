use codec::{Decode, Encode, Input, Output};
use rstd::collections::btree_map::BTreeMap;

#[derive(Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct CodecBTreeMap<K: Ord, V>(pub BTreeMap<K, V>);

impl<K: Encode + Ord, V: Encode> Encode for CodecBTreeMap<K, V> {
    fn encode_to<W: Output>(&self, dest: &mut W) {
        let len = self.0.len();
        assert!(
            len <= u32::max_value() as usize,
            "Attempted to serialize a collection with too many elements."
        );
        (len as u32).encode_to(dest);
        for i in self.0.iter() {
            i.encode_to(dest);
        }
    }
}

impl<K: Decode + Ord, V: Decode> Decode for CodecBTreeMap<K, V> {
    fn decode<I: Input>(input: &mut I) -> Option<Self> {
        u32::decode(input).and_then(move |len| {
            let mut r: BTreeMap<K, V> = BTreeMap::new(); // with_capacity(len as usize);
            for _ in 0..len {
                let (key, v) = <(K, V)>::decode(input)?;
                r.insert(key, v);
            }
            Some(CodecBTreeMap::<K, V>(r))
        })
    }
}
