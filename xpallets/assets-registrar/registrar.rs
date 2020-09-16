#![feature(prelude_import)]
//! This crate provides the feature of managing the native and foreign assets' meta information.
//!
//! The foreign asset hereby means it's not the native token of the system(PCX for ChainX)
//! but derived from the other blockchain system, e.g., Bitcoin.
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
mod default_weights {
    use frame_support::weights::Weight;
    use crate::WeightInfo;
    impl WeightInfo for () {
        fn register() -> Weight {
            1_000_000_000
        }
        fn deregister() -> Weight {
            1_000_000_000
        }
        fn recover() -> Weight {
            1_000_000_000
        }
        fn update_asset_info() -> Weight {
            1_000_000_000
        }
    }
}
mod types {
    use sp_std::{fmt, result, slice::Iter};
    use codec::{Decode, Encode};
    use frame_support::{
        dispatch::{DispatchError, DispatchResult},
        RuntimeDebug,
    };
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use chainx_primitives::{Decimals, Desc, Token};
    use crate::verifier::*;
    use crate::Trait;
    pub enum Chain {
        ChainX,
        Bitcoin,
        Ethereum,
        Polkadot,
    }
    impl ::core::marker::StructuralPartialEq for Chain {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for Chain {
        #[inline]
        fn eq(&self, other: &Chain) -> bool {
            {
                let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
                let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => true,
                    }
                } else {
                    false
                }
            }
        }
    }
    impl ::core::marker::StructuralEq for Chain {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for Chain {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () {
            {}
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Ord for Chain {
        #[inline]
        fn cmp(&self, other: &Chain) -> ::core::cmp::Ordering {
            {
                let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
                let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => ::core::cmp::Ordering::Equal,
                    }
                } else {
                    __self_vi.cmp(&__arg_1_vi)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialOrd for Chain {
        #[inline]
        fn partial_cmp(&self, other: &Chain) -> ::core::option::Option<::core::cmp::Ordering> {
            {
                let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
                let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => ::core::option::Option::Some(::core::cmp::Ordering::Equal),
                    }
                } else {
                    __self_vi.partial_cmp(&__arg_1_vi)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Chain {
        #[inline]
        fn clone(&self) -> Chain {
            {
                *self
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for Chain {}
    const _: () = {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate codec as _parity_scale_codec;
        impl _parity_scale_codec::Encode for Chain {
            fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
                &self,
                __codec_dest_edqy: &mut __CodecOutputEdqy,
            ) {
                match *self {
                    Chain::ChainX => {
                        __codec_dest_edqy.push_byte(0usize as u8);
                    }
                    Chain::Bitcoin => {
                        __codec_dest_edqy.push_byte(1usize as u8);
                    }
                    Chain::Ethereum => {
                        __codec_dest_edqy.push_byte(2usize as u8);
                    }
                    Chain::Polkadot => {
                        __codec_dest_edqy.push_byte(3usize as u8);
                    }
                    _ => (),
                }
            }
        }
        impl _parity_scale_codec::EncodeLike for Chain {}
    };
    const _: () = {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate codec as _parity_scale_codec;
        impl _parity_scale_codec::Decode for Chain {
            fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
                __codec_input_edqy: &mut __CodecInputEdqy,
            ) -> core::result::Result<Self, _parity_scale_codec::Error> {
                match __codec_input_edqy.read_byte()? {
                    __codec_x_edqy if __codec_x_edqy == 0usize as u8 => Ok(Chain::ChainX),
                    __codec_x_edqy if __codec_x_edqy == 1usize as u8 => Ok(Chain::Bitcoin),
                    __codec_x_edqy if __codec_x_edqy == 2usize as u8 => Ok(Chain::Ethereum),
                    __codec_x_edqy if __codec_x_edqy == 3usize as u8 => Ok(Chain::Polkadot),
                    _ => Err("No such variant in enum Chain".into()),
                }
            }
        }
    };
    impl core::fmt::Debug for Chain {
        fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                Self::ChainX => fmt.debug_tuple("Chain::ChainX").finish(),
                Self::Bitcoin => fmt.debug_tuple("Chain::Bitcoin").finish(),
                Self::Ethereum => fmt.debug_tuple("Chain::Ethereum").finish(),
                Self::Polkadot => fmt.debug_tuple("Chain::Polkadot").finish(),
                _ => Ok(()),
            }
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(rust_2018_idioms, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for Chain {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::export::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                match *self {
                    Chain::ChainX => _serde::Serializer::serialize_unit_variant(
                        __serializer,
                        "Chain",
                        0u32,
                        "ChainX",
                    ),
                    Chain::Bitcoin => _serde::Serializer::serialize_unit_variant(
                        __serializer,
                        "Chain",
                        1u32,
                        "Bitcoin",
                    ),
                    Chain::Ethereum => _serde::Serializer::serialize_unit_variant(
                        __serializer,
                        "Chain",
                        2u32,
                        "Ethereum",
                    ),
                    Chain::Polkadot => _serde::Serializer::serialize_unit_variant(
                        __serializer,
                        "Chain",
                        3u32,
                        "Polkadot",
                    ),
                }
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(rust_2018_idioms, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for Chain {
            fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                }
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::export::Formatter,
                    ) -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter, "variant identifier")
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::export::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::export::Ok(__Field::__field0),
                            1u64 => _serde::export::Ok(__Field::__field1),
                            2u64 => _serde::export::Ok(__Field::__field2),
                            3u64 => _serde::export::Ok(__Field::__field3),
                            _ => _serde::export::Err(_serde::de::Error::invalid_value(
                                _serde::de::Unexpected::Unsigned(__value),
                                &"variant index 0 <= i < 4",
                            )),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::export::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "ChainX" => _serde::export::Ok(__Field::__field0),
                            "Bitcoin" => _serde::export::Ok(__Field::__field1),
                            "Ethereum" => _serde::export::Ok(__Field::__field2),
                            "Polkadot" => _serde::export::Ok(__Field::__field3),
                            _ => _serde::export::Err(_serde::de::Error::unknown_variant(
                                __value, VARIANTS,
                            )),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::export::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"ChainX" => _serde::export::Ok(__Field::__field0),
                            b"Bitcoin" => _serde::export::Ok(__Field::__field1),
                            b"Ethereum" => _serde::export::Ok(__Field::__field2),
                            b"Polkadot" => _serde::export::Ok(__Field::__field3),
                            _ => {
                                let __value = &_serde::export::from_utf8_lossy(__value);
                                _serde::export::Err(_serde::de::Error::unknown_variant(
                                    __value, VARIANTS,
                                ))
                            }
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::export::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<Chain>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = Chain;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::export::Formatter,
                    ) -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter, "enum Chain")
                    }
                    fn visit_enum<__A>(
                        self,
                        __data: __A,
                    ) -> _serde::export::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::EnumAccess<'de>,
                    {
                        match match _serde::de::EnumAccess::variant(__data) {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        } {
                            (__Field::__field0, __variant) => {
                                match _serde::de::VariantAccess::unit_variant(__variant) {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                };
                                _serde::export::Ok(Chain::ChainX)
                            }
                            (__Field::__field1, __variant) => {
                                match _serde::de::VariantAccess::unit_variant(__variant) {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                };
                                _serde::export::Ok(Chain::Bitcoin)
                            }
                            (__Field::__field2, __variant) => {
                                match _serde::de::VariantAccess::unit_variant(__variant) {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                };
                                _serde::export::Ok(Chain::Ethereum)
                            }
                            (__Field::__field3, __variant) => {
                                match _serde::de::VariantAccess::unit_variant(__variant) {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                };
                                _serde::export::Ok(Chain::Polkadot)
                            }
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] =
                    &["ChainX", "Bitcoin", "Ethereum", "Polkadot"];
                _serde::Deserializer::deserialize_enum(
                    __deserializer,
                    "Chain",
                    VARIANTS,
                    __Visitor {
                        marker: _serde::export::PhantomData::<Chain>,
                        lifetime: _serde::export::PhantomData,
                    },
                )
            }
        }
    };
    const CHAINS: [Chain; 4] = [
        Chain::ChainX,
        Chain::Bitcoin,
        Chain::Ethereum,
        Chain::Polkadot,
    ];
    impl Chain {
        /// Returns an iterator of all `Chain`.
        pub fn iter() -> Iter<'static, Chain> {
            CHAINS.iter()
        }
    }
    impl Default for Chain {
        fn default() -> Self {
            Chain::ChainX
        }
    }
    #[serde(rename_all = "camelCase")]
    pub struct AssetInfo {
        token: Token,
        token_name: Token,
        chain: Chain,
        decimals: Decimals,
        desc: Desc,
    }
    impl ::core::marker::StructuralPartialEq for AssetInfo {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::PartialEq for AssetInfo {
        #[inline]
        fn eq(&self, other: &AssetInfo) -> bool {
            match *other {
                AssetInfo {
                    token: ref __self_1_0,
                    token_name: ref __self_1_1,
                    chain: ref __self_1_2,
                    decimals: ref __self_1_3,
                    desc: ref __self_1_4,
                } => match *self {
                    AssetInfo {
                        token: ref __self_0_0,
                        token_name: ref __self_0_1,
                        chain: ref __self_0_2,
                        decimals: ref __self_0_3,
                        desc: ref __self_0_4,
                    } => {
                        (*__self_0_0) == (*__self_1_0)
                            && (*__self_0_1) == (*__self_1_1)
                            && (*__self_0_2) == (*__self_1_2)
                            && (*__self_0_3) == (*__self_1_3)
                            && (*__self_0_4) == (*__self_1_4)
                    }
                },
            }
        }
        #[inline]
        fn ne(&self, other: &AssetInfo) -> bool {
            match *other {
                AssetInfo {
                    token: ref __self_1_0,
                    token_name: ref __self_1_1,
                    chain: ref __self_1_2,
                    decimals: ref __self_1_3,
                    desc: ref __self_1_4,
                } => match *self {
                    AssetInfo {
                        token: ref __self_0_0,
                        token_name: ref __self_0_1,
                        chain: ref __self_0_2,
                        decimals: ref __self_0_3,
                        desc: ref __self_0_4,
                    } => {
                        (*__self_0_0) != (*__self_1_0)
                            || (*__self_0_1) != (*__self_1_1)
                            || (*__self_0_2) != (*__self_1_2)
                            || (*__self_0_3) != (*__self_1_3)
                            || (*__self_0_4) != (*__self_1_4)
                    }
                },
            }
        }
    }
    impl ::core::marker::StructuralEq for AssetInfo {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::cmp::Eq for AssetInfo {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () {
            {
                let _: ::core::cmp::AssertParamIsEq<Token>;
                let _: ::core::cmp::AssertParamIsEq<Token>;
                let _: ::core::cmp::AssertParamIsEq<Chain>;
                let _: ::core::cmp::AssertParamIsEq<Decimals>;
                let _: ::core::cmp::AssertParamIsEq<Desc>;
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for AssetInfo {
        #[inline]
        fn clone(&self) -> AssetInfo {
            match *self {
                AssetInfo {
                    token: ref __self_0_0,
                    token_name: ref __self_0_1,
                    chain: ref __self_0_2,
                    decimals: ref __self_0_3,
                    desc: ref __self_0_4,
                } => AssetInfo {
                    token: ::core::clone::Clone::clone(&(*__self_0_0)),
                    token_name: ::core::clone::Clone::clone(&(*__self_0_1)),
                    chain: ::core::clone::Clone::clone(&(*__self_0_2)),
                    decimals: ::core::clone::Clone::clone(&(*__self_0_3)),
                    desc: ::core::clone::Clone::clone(&(*__self_0_4)),
                },
            }
        }
    }
    const _: () = {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate codec as _parity_scale_codec;
        impl _parity_scale_codec::Encode for AssetInfo {
            fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
                &self,
                __codec_dest_edqy: &mut __CodecOutputEdqy,
            ) {
                __codec_dest_edqy.push(&self.token);
                __codec_dest_edqy.push(&self.token_name);
                __codec_dest_edqy.push(&self.chain);
                __codec_dest_edqy.push(&self.decimals);
                __codec_dest_edqy.push(&self.desc);
            }
        }
        impl _parity_scale_codec::EncodeLike for AssetInfo {}
    };
    const _: () = {
        #[allow(unknown_lints)]
        #[allow(rust_2018_idioms)]
        extern crate codec as _parity_scale_codec;
        impl _parity_scale_codec::Decode for AssetInfo {
            fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
                __codec_input_edqy: &mut __CodecInputEdqy,
            ) -> core::result::Result<Self, _parity_scale_codec::Error> {
                Ok(AssetInfo {
                    token: {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field AssetInfo.token".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    token_name: {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => {
                                return Err("Error decoding field AssetInfo.token_name".into())
                            }
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    chain: {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field AssetInfo.chain".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    decimals: {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field AssetInfo.decimals".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    desc: {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field AssetInfo.desc".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                })
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(rust_2018_idioms, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for AssetInfo {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::export::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = match _serde::Serializer::serialize_struct(
                    __serializer,
                    "AssetInfo",
                    false as usize + 1 + 1 + 1 + 1 + 1,
                ) {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "token",
                    &self.token,
                ) {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "tokenName",
                    &self.token_name,
                ) {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "chain",
                    &self.chain,
                ) {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "decimals",
                    &self.decimals,
                ) {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                match _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "desc",
                    &self.desc,
                ) {
                    _serde::export::Ok(__val) => __val,
                    _serde::export::Err(__err) => {
                        return _serde::export::Err(__err);
                    }
                };
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(rust_2018_idioms, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for AssetInfo {
            fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                    __field4,
                    __ignore,
                }
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::export::Formatter,
                    ) -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter, "field identifier")
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::export::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::export::Ok(__Field::__field0),
                            1u64 => _serde::export::Ok(__Field::__field1),
                            2u64 => _serde::export::Ok(__Field::__field2),
                            3u64 => _serde::export::Ok(__Field::__field3),
                            4u64 => _serde::export::Ok(__Field::__field4),
                            _ => _serde::export::Err(_serde::de::Error::invalid_value(
                                _serde::de::Unexpected::Unsigned(__value),
                                &"field index 0 <= i < 5",
                            )),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::export::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "token" => _serde::export::Ok(__Field::__field0),
                            "tokenName" => _serde::export::Ok(__Field::__field1),
                            "chain" => _serde::export::Ok(__Field::__field2),
                            "decimals" => _serde::export::Ok(__Field::__field3),
                            "desc" => _serde::export::Ok(__Field::__field4),
                            _ => _serde::export::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::export::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"token" => _serde::export::Ok(__Field::__field0),
                            b"tokenName" => _serde::export::Ok(__Field::__field1),
                            b"chain" => _serde::export::Ok(__Field::__field2),
                            b"decimals" => _serde::export::Ok(__Field::__field3),
                            b"desc" => _serde::export::Ok(__Field::__field4),
                            _ => _serde::export::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::export::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                    }
                }
                struct __Visitor<'de> {
                    marker: _serde::export::PhantomData<AssetInfo>,
                    lifetime: _serde::export::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = AssetInfo;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::export::Formatter,
                    ) -> _serde::export::fmt::Result {
                        _serde::export::Formatter::write_str(__formatter, "struct AssetInfo")
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::export::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 =
                            match match _serde::de::SeqAccess::next_element::<Token>(&mut __seq) {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct AssetInfo with 5 elements",
                                    ));
                                }
                            };
                        let __field1 =
                            match match _serde::de::SeqAccess::next_element::<Token>(&mut __seq) {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct AssetInfo with 5 elements",
                                    ));
                                }
                            };
                        let __field2 =
                            match match _serde::de::SeqAccess::next_element::<Chain>(&mut __seq) {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(
                                        2usize,
                                        &"struct AssetInfo with 5 elements",
                                    ));
                                }
                            };
                        let __field3 =
                            match match _serde::de::SeqAccess::next_element::<Decimals>(&mut __seq)
                            {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(
                                        3usize,
                                        &"struct AssetInfo with 5 elements",
                                    ));
                                }
                            };
                        let __field4 =
                            match match _serde::de::SeqAccess::next_element::<Desc>(&mut __seq) {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            } {
                                _serde::export::Some(__value) => __value,
                                _serde::export::None => {
                                    return _serde::export::Err(_serde::de::Error::invalid_length(
                                        4usize,
                                        &"struct AssetInfo with 5 elements",
                                    ));
                                }
                            };
                        _serde::export::Ok(AssetInfo {
                            token: __field0,
                            token_name: __field1,
                            chain: __field2,
                            decimals: __field3,
                            desc: __field4,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::export::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::export::Option<Token> = _serde::export::None;
                        let mut __field1: _serde::export::Option<Token> = _serde::export::None;
                        let mut __field2: _serde::export::Option<Chain> = _serde::export::None;
                        let mut __field3: _serde::export::Option<Decimals> = _serde::export::None;
                        let mut __field4: _serde::export::Option<Desc> = _serde::export::None;
                        while let _serde::export::Some(__key) =
                            match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::export::Option::is_some(&__field0) {
                                        return _serde::export::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "token",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::export::Some(
                                        match _serde::de::MapAccess::next_value::<Token>(&mut __map)
                                        {
                                            _serde::export::Ok(__val) => __val,
                                            _serde::export::Err(__err) => {
                                                return _serde::export::Err(__err);
                                            }
                                        },
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::export::Option::is_some(&__field1) {
                                        return _serde::export::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "tokenName",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::export::Some(
                                        match _serde::de::MapAccess::next_value::<Token>(&mut __map)
                                        {
                                            _serde::export::Ok(__val) => __val,
                                            _serde::export::Err(__err) => {
                                                return _serde::export::Err(__err);
                                            }
                                        },
                                    );
                                }
                                __Field::__field2 => {
                                    if _serde::export::Option::is_some(&__field2) {
                                        return _serde::export::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "chain",
                                            ),
                                        );
                                    }
                                    __field2 = _serde::export::Some(
                                        match _serde::de::MapAccess::next_value::<Chain>(&mut __map)
                                        {
                                            _serde::export::Ok(__val) => __val,
                                            _serde::export::Err(__err) => {
                                                return _serde::export::Err(__err);
                                            }
                                        },
                                    );
                                }
                                __Field::__field3 => {
                                    if _serde::export::Option::is_some(&__field3) {
                                        return _serde::export::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "decimals",
                                            ),
                                        );
                                    }
                                    __field3 = _serde::export::Some(
                                        match _serde::de::MapAccess::next_value::<Decimals>(
                                            &mut __map,
                                        ) {
                                            _serde::export::Ok(__val) => __val,
                                            _serde::export::Err(__err) => {
                                                return _serde::export::Err(__err);
                                            }
                                        },
                                    );
                                }
                                __Field::__field4 => {
                                    if _serde::export::Option::is_some(&__field4) {
                                        return _serde::export::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "desc",
                                            ),
                                        );
                                    }
                                    __field4 = _serde::export::Some(
                                        match _serde::de::MapAccess::next_value::<Desc>(&mut __map)
                                        {
                                            _serde::export::Ok(__val) => __val,
                                            _serde::export::Err(__err) => {
                                                return _serde::export::Err(__err);
                                            }
                                        },
                                    );
                                }
                                _ => {
                                    let _ = match _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)
                                    {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    };
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::export::Some(__field0) => __field0,
                            _serde::export::None => {
                                match _serde::private::de::missing_field("token") {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                }
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::export::Some(__field1) => __field1,
                            _serde::export::None => {
                                match _serde::private::de::missing_field("tokenName") {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                }
                            }
                        };
                        let __field2 = match __field2 {
                            _serde::export::Some(__field2) => __field2,
                            _serde::export::None => {
                                match _serde::private::de::missing_field("chain") {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                }
                            }
                        };
                        let __field3 = match __field3 {
                            _serde::export::Some(__field3) => __field3,
                            _serde::export::None => {
                                match _serde::private::de::missing_field("decimals") {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                }
                            }
                        };
                        let __field4 = match __field4 {
                            _serde::export::Some(__field4) => __field4,
                            _serde::export::None => {
                                match _serde::private::de::missing_field("desc") {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                }
                            }
                        };
                        _serde::export::Ok(AssetInfo {
                            token: __field0,
                            token_name: __field1,
                            chain: __field2,
                            decimals: __field3,
                            desc: __field4,
                        })
                    }
                }
                const FIELDS: &'static [&'static str] =
                    &["token", "tokenName", "chain", "decimals", "desc"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "AssetInfo",
                    FIELDS,
                    __Visitor {
                        marker: _serde::export::PhantomData::<AssetInfo>,
                        lifetime: _serde::export::PhantomData,
                    },
                )
            }
        }
    };
    impl fmt::Debug for AssetInfo {
        #[cfg(feature = "std")]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(::core::fmt::Arguments::new_v1(
                &[
                    "AssetInfo: {token: ",
                    ", token_name: ",
                    ", chain: ",
                    ", decimals: ",
                    ", desc: ",
                    "}",
                ],
                &match (
                    &String::from_utf8_lossy(&self.token).into_owned(),
                    &String::from_utf8_lossy(&self.token_name).into_owned(),
                    &self.chain,
                    &self.decimals,
                    &String::from_utf8_lossy(&self.desc).into_owned(),
                ) {
                    (arg0, arg1, arg2, arg3, arg4) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg2, ::core::fmt::Debug::fmt),
                        ::core::fmt::ArgumentV1::new(arg3, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg4, ::core::fmt::Display::fmt),
                    ],
                },
            ))
        }
    }
    impl AssetInfo {
        pub fn new<T: Trait>(
            token: Token,
            token_name: Token,
            chain: Chain,
            decimals: Decimals,
            desc: Desc,
        ) -> result::Result<Self, DispatchError> {
            let asset = AssetInfo {
                token,
                token_name,
                chain,
                decimals,
                desc,
            };
            asset.is_valid::<T>()?;
            Ok(asset)
        }
        pub fn is_valid<T: Trait>(&self) -> DispatchResult {
            is_valid_token::<T>(&self.token)?;
            is_valid_token_name::<T>(&self.token_name)?;
            is_valid_desc::<T>(&self.desc)
        }
        pub fn token(&self) -> &Token {
            &self.token
        }
        pub fn token_name(&self) -> &Token {
            &self.token_name
        }
        pub fn chain(&self) -> Chain {
            self.chain
        }
        pub fn desc(&self) -> &Desc {
            &self.desc
        }
        pub fn decimals(&self) -> Decimals {
            self.decimals
        }
        pub fn set_desc(&mut self, desc: Desc) {
            self.desc = desc
        }
        pub fn set_token(&mut self, token: Token) {
            self.token = token
        }
        pub fn set_token_name(&mut self, token_name: Token) {
            self.token_name = token_name
        }
    }
}
mod verifier {
    use xpallet_protocol::{ASSET_DESC_MAX_LEN, ASSET_TOKEN_NAME_MAX_LEN, ASSET_TOKEN_SYMBOL_MAX_LEN};
    use super::*;
    /// Token can only use ASCII alphanumeric character or "-.|~".
    pub fn is_valid_token<T: Trait>(token: &[u8]) -> DispatchResult {
        if token.len() > ASSET_TOKEN_SYMBOL_MAX_LEN || token.is_empty() {
            return Err(Error::<T>::InvalidAssetTokenSymbolLength.into());
        }
        let is_valid = |c: &u8| -> bool { c.is_ascii_alphanumeric() || b"-.|~".contains(c) };
        for c in token {
            if !is_valid(c) {
                return Err(Error::<T>::InvalidAssetTokenSymbolChar.into());
            }
        }
        Ok(())
    }
    /// A valid token name should have a legal length and be visible ASCII chars only.
    pub fn is_valid_token_name<T: Trait>(token_name: &[u8]) -> DispatchResult {
        if token_name.len() > ASSET_TOKEN_NAME_MAX_LEN || token_name.is_empty() {
            return Err(Error::<T>::InvalidAssetTokenNameLength.into());
        }
        xp_runtime::xss_check(token_name)?;
        for c in token_name {
            if !is_ascii_visible(c) {
                return Err(Error::<T>::InvalidAscii.into());
            }
        }
        Ok(())
    }
    /// A valid desc should be visible ASCII chars only and not too long.
    pub fn is_valid_desc<T: Trait>(desc: &[u8]) -> DispatchResult {
        if desc.len() > ASSET_DESC_MAX_LEN {
            return Err(Error::<T>::InvalidAssetDescLength.into());
        }
        xp_runtime::xss_check(desc)?;
        for c in desc {
            if !is_ascii_visible(c) {
                return Err(Error::<T>::InvalidAscii.into());
            }
        }
        Ok(())
    }
    /// Visible ASCII char [0x20, 0x7E]
    #[inline]
    fn is_ascii_visible(c: &u8) -> bool {
        *c == b' ' || c.is_ascii_graphic()
    }
}
use sp_std::{prelude::*, result};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    traits::Get,
    weights::Weight,
    IterableStorageMap,
};
use frame_system::ensure_root;
use chainx_primitives::{AssetId, Desc, Token};
use xpallet_support::info;
pub use self::types::{AssetInfo, Chain};
pub use xp_assets_registrar::RegistrarHandler;
/// Weight information for extrinsics in this pallet.
pub trait WeightInfo {
    fn register() -> Weight;
    fn deregister() -> Weight;
    fn recover() -> Weight;
    fn update_asset_info() -> Weight;
}
/// The module's config trait.
///
/// `frame_system::Trait` should always be included in our implied traits.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
    /// Native asset Id.
    type NativeAssetId: Get<AssetId>;
    /// Handler for doing stuff after the asset is registered/deregistered.
    type RegistrarHandler: RegistrarHandler;
    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}
/// Events for this module.
///
/// Event for the XAssetRegistrar Module
pub enum Event {
    /// A new asset is registered. [asset_id, has_mining_rights]
    Register(AssetId, bool),
    /// A deregistered asset is recovered. [asset_id, has_mining_rights]
    Recover(AssetId, bool),
    /// An asset is invalid now. [asset_id]
    Deregister(AssetId),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Event {
    #[inline]
    fn clone(&self) -> Event {
        match (&*self,) {
            (&Event::Register(ref __self_0, ref __self_1),) => Event::Register(
                ::core::clone::Clone::clone(&(*__self_0)),
                ::core::clone::Clone::clone(&(*__self_1)),
            ),
            (&Event::Recover(ref __self_0, ref __self_1),) => Event::Recover(
                ::core::clone::Clone::clone(&(*__self_0)),
                ::core::clone::Clone::clone(&(*__self_1)),
            ),
            (&Event::Deregister(ref __self_0),) => {
                Event::Deregister(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
impl ::core::marker::StructuralPartialEq for Event {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for Event {
    #[inline]
    fn eq(&self, other: &Event) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (
                        &Event::Register(ref __self_0, ref __self_1),
                        &Event::Register(ref __arg_1_0, ref __arg_1_1),
                    ) => (*__self_0) == (*__arg_1_0) && (*__self_1) == (*__arg_1_1),
                    (
                        &Event::Recover(ref __self_0, ref __self_1),
                        &Event::Recover(ref __arg_1_0, ref __arg_1_1),
                    ) => (*__self_0) == (*__arg_1_0) && (*__self_1) == (*__arg_1_1),
                    (&Event::Deregister(ref __self_0), &Event::Deregister(ref __arg_1_0)) => {
                        (*__self_0) == (*__arg_1_0)
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() },
                }
            } else {
                false
            }
        }
    }
    #[inline]
    fn ne(&self, other: &Event) -> bool {
        {
            let __self_vi = unsafe { ::core::intrinsics::discriminant_value(&*self) };
            let __arg_1_vi = unsafe { ::core::intrinsics::discriminant_value(&*other) };
            if true && __self_vi == __arg_1_vi {
                match (&*self, &*other) {
                    (
                        &Event::Register(ref __self_0, ref __self_1),
                        &Event::Register(ref __arg_1_0, ref __arg_1_1),
                    ) => (*__self_0) != (*__arg_1_0) || (*__self_1) != (*__arg_1_1),
                    (
                        &Event::Recover(ref __self_0, ref __self_1),
                        &Event::Recover(ref __arg_1_0, ref __arg_1_1),
                    ) => (*__self_0) != (*__arg_1_0) || (*__self_1) != (*__arg_1_1),
                    (&Event::Deregister(ref __self_0), &Event::Deregister(ref __arg_1_0)) => {
                        (*__self_0) != (*__arg_1_0)
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() },
                }
            } else {
                true
            }
        }
    }
}
impl ::core::marker::StructuralEq for Event {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for Event {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
            let _: ::core::cmp::AssertParamIsEq<AssetId>;
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for Event {
        fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
            &self,
            __codec_dest_edqy: &mut __CodecOutputEdqy,
        ) {
            match *self {
                Event::Register(ref aa, ref ba) => {
                    __codec_dest_edqy.push_byte(0usize as u8);
                    __codec_dest_edqy.push(aa);
                    __codec_dest_edqy.push(ba);
                }
                Event::Recover(ref aa, ref ba) => {
                    __codec_dest_edqy.push_byte(1usize as u8);
                    __codec_dest_edqy.push(aa);
                    __codec_dest_edqy.push(ba);
                }
                Event::Deregister(ref aa) => {
                    __codec_dest_edqy.push_byte(2usize as u8);
                    __codec_dest_edqy.push(aa);
                }
                _ => (),
            }
        }
    }
    impl _parity_scale_codec::EncodeLike for Event {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for Event {
        fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
            __codec_input_edqy: &mut __CodecInputEdqy,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match __codec_input_edqy.read_byte()? {
                __codec_x_edqy if __codec_x_edqy == 0usize as u8 => Ok(Event::Register(
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Event :: Register.0".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Event :: Register.1".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                )),
                __codec_x_edqy if __codec_x_edqy == 1usize as u8 => Ok(Event::Recover(
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Event :: Recover.0".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Event :: Recover.1".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                )),
                __codec_x_edqy if __codec_x_edqy == 2usize as u8 => Ok(Event::Deregister({
                    let __codec_res_edqy = _parity_scale_codec::Decode::decode(__codec_input_edqy);
                    match __codec_res_edqy {
                        Err(_) => return Err("Error decoding field Event :: Deregister.0".into()),
                        Ok(__codec_res_edqy) => __codec_res_edqy,
                    }
                })),
                _ => Err("No such variant in enum Event".into()),
            }
        }
    }
};
impl core::fmt::Debug for Event {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Register(ref a0, ref a1) => fmt
                .debug_tuple("Event::Register")
                .field(a0)
                .field(a1)
                .finish(),
            Self::Recover(ref a0, ref a1) => fmt
                .debug_tuple("Event::Recover")
                .field(a0)
                .field(a1)
                .finish(),
            Self::Deregister(ref a0) => fmt.debug_tuple("Event::Deregister").field(a0).finish(),
            _ => Ok(()),
        }
    }
}
impl From<Event> for () {
    fn from(_: Event) -> () {
        ()
    }
}
impl Event {
    #[allow(dead_code)]
    #[doc(hidden)]
    pub fn metadata() -> &'static [::frame_support::event::EventMetadata] {
        &[
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("Register"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["AssetId", "bool"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[
                    r" A new asset is registered. [asset_id, has_mining_rights]",
                ]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("Recover"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["AssetId", "bool"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[
                    r" A deregistered asset is recovered. [asset_id, has_mining_rights]",
                ]),
            },
            ::frame_support::event::EventMetadata {
                name: ::frame_support::event::DecodeDifferent::Encode("Deregister"),
                arguments: ::frame_support::event::DecodeDifferent::Encode(&["AssetId"]),
                documentation: ::frame_support::event::DecodeDifferent::Encode(&[
                    r" An asset is invalid now. [asset_id]",
                ]),
            },
        ]
    }
}
/// Error for the XAssetRegistrar Module
pub enum Error<T: Trait> {
    #[doc(hidden)]
    __Ignore(
        ::frame_support::sp_std::marker::PhantomData<(T,)>,
        ::frame_support::Never,
    ),
    /// Token symbol length is zero or too long
    InvalidAssetTokenSymbolLength,
    /// Token symbol char is invalid, only allow ASCII alphanumeric character or '-', '.', '|', '~'
    InvalidAssetTokenSymbolChar,
    /// Token name length is zero or too long
    InvalidAssetTokenNameLength,
    /// Desc length is zero or too long
    InvalidAssetDescLength,
    /// Text is invalid ASCII, only allow ASCII visible character [0x20, 0x7E]
    InvalidAscii,
    /// The asset already exists.
    AssetAlreadyExists,
    /// The asset is not exist.
    AssetIsNotExist,
    /// The asset is already valid (online), no need to recover.
    AssetAlreadyValid,
    /// The asset is invalid (not online).
    AssetIsInvalid,
}
impl<T: Trait> ::frame_support::sp_std::fmt::Debug for Error<T> {
    fn fmt(
        &self,
        f: &mut ::frame_support::sp_std::fmt::Formatter<'_>,
    ) -> ::frame_support::sp_std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl<T: Trait> Error<T> {
    fn as_u8(&self) -> u8 {
        match self {
            Error::__Ignore(_, _) => ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                &["internal error: entered unreachable code: "],
                &match (&"`__Ignore` can never be constructed",) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            )),
            Error::InvalidAssetTokenSymbolLength => 0,
            Error::InvalidAssetTokenSymbolChar => 0 + 1,
            Error::InvalidAssetTokenNameLength => 0 + 1 + 1,
            Error::InvalidAssetDescLength => 0 + 1 + 1 + 1,
            Error::InvalidAscii => 0 + 1 + 1 + 1 + 1,
            Error::AssetAlreadyExists => 0 + 1 + 1 + 1 + 1 + 1,
            Error::AssetIsNotExist => 0 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::AssetAlreadyValid => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            Error::AssetIsInvalid => 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Self::__Ignore(_, _) => ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                &["internal error: entered unreachable code: "],
                &match (&"`__Ignore` can never be constructed",) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            )),
            Error::InvalidAssetTokenSymbolLength => "InvalidAssetTokenSymbolLength",
            Error::InvalidAssetTokenSymbolChar => "InvalidAssetTokenSymbolChar",
            Error::InvalidAssetTokenNameLength => "InvalidAssetTokenNameLength",
            Error::InvalidAssetDescLength => "InvalidAssetDescLength",
            Error::InvalidAscii => "InvalidAscii",
            Error::AssetAlreadyExists => "AssetAlreadyExists",
            Error::AssetIsNotExist => "AssetIsNotExist",
            Error::AssetAlreadyValid => "AssetAlreadyValid",
            Error::AssetIsInvalid => "AssetIsInvalid",
        }
    }
}
impl<T: Trait> From<Error<T>> for &'static str {
    fn from(err: Error<T>) -> &'static str {
        err.as_str()
    }
}
impl<T: Trait> From<Error<T>> for ::frame_support::sp_runtime::DispatchError {
    fn from(err: Error<T>) -> Self {
        let index = <T::ModuleToIndex as ::frame_support::traits::ModuleToIndex>::module_to_index::<
            Module<T>,
        >()
        .expect("Every active module has an index in the runtime; qed") as u8;
        ::frame_support::sp_runtime::DispatchError::Module {
            index,
            error: err.as_u8(),
            message: Some(err.as_str()),
        }
    }
}
impl<T: Trait> ::frame_support::error::ModuleErrorMetadata for Error<T> {
    fn metadata() -> &'static [::frame_support::error::ErrorMetadata] {
        &[
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode(
                    "InvalidAssetTokenSymbolLength",
                ),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" Token symbol length is zero or too long",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode(
                    "InvalidAssetTokenSymbolChar",
                ),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" Token symbol char is invalid, only allow ASCII alphanumeric character or '-', '.', '|', '~'",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode(
                    "InvalidAssetTokenNameLength",
                ),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" Token name length is zero or too long",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("InvalidAssetDescLength"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" Desc length is zero or too long",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("InvalidAscii"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" Text is invalid ASCII, only allow ASCII visible character [0x20, 0x7E]",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("AssetAlreadyExists"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" The asset already exists.",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("AssetIsNotExist"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" The asset is not exist.",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("AssetAlreadyValid"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" The asset is already valid (online), no need to recover.",
                ]),
            },
            ::frame_support::error::ErrorMetadata {
                name: ::frame_support::error::DecodeDifferent::Encode("AssetIsInvalid"),
                documentation: ::frame_support::error::DecodeDifferent::Encode(&[
                    r" The asset is invalid (not online).",
                ]),
            },
        ]
    }
}
use self::sp_api_hidden_includes_decl_storage::hidden_include::{
    StorageValue as _, StorageMap as _, StorageDoubleMap as _, StoragePrefixedMap as _,
    IterableStorageMap as _, IterableStorageDoubleMap as _,
};
#[doc(hidden)]
mod sp_api_hidden_includes_decl_storage {
    pub extern crate frame_support as hidden_include;
}
trait Store {
    type AssetIdsOf;
    type AssetInfoOf;
    type AssetOnline;
    type RegisteredAt;
}
impl<T: Trait + 'static> Store for Module<T> {
    type AssetIdsOf = AssetIdsOf;
    type AssetInfoOf = AssetInfoOf;
    type AssetOnline = AssetOnline;
    type RegisteredAt = RegisteredAt<T>;
}
impl<T: Trait + 'static> Module<T> {
    /// Asset id list for each Chain.
    pub fn asset_ids_of<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<Chain>,
    >(
        key: K,
    ) -> Vec<AssetId> {
        < AssetIdsOf < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < Chain , Vec < AssetId > > > :: get ( key )
    }
    /// Asset info of each asset.
    pub fn asset_info_of<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<AssetId>,
    >(
        key: K,
    ) -> Option<AssetInfo> {
        < AssetInfoOf < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < AssetId , AssetInfo > > :: get ( key )
    }
    /// The map of asset to the online state.
    pub fn asset_online<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<AssetId>,
    >(
        key: K,
    ) -> Option<()> {
        < AssetOnline < > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < AssetId , ( ) > > :: get ( key )
    }
    /// The map of asset to the block number at which the asset was registered.
    pub fn registered_at<
        K: self::sp_api_hidden_includes_decl_storage::hidden_include::codec::EncodeLike<AssetId>,
    >(
        key: K,
    ) -> T::BlockNumber {
        < RegisteredAt < T > as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: storage :: StorageMap < AssetId , T :: BlockNumber > > :: get ( key )
    }
}
#[doc(hidden)]
pub struct __GetByteStructAssetIdsOf<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_AssetIdsOf:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructAssetIdsOf<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_AssetIdsOf
            .get_or_init(|| {
                let def_val: Vec<AssetId> = Default::default();
                <Vec<AssetId> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructAssetIdsOf<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructAssetIdsOf<T> {}
#[doc(hidden)]
pub struct __GetByteStructAssetInfoOf<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_AssetInfoOf:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructAssetInfoOf<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_AssetInfoOf
            .get_or_init(|| {
                let def_val: Option<AssetInfo> = Default::default();
                <Option<AssetInfo> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructAssetInfoOf<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructAssetInfoOf<T> {}
#[doc(hidden)]
pub struct __GetByteStructAssetOnline<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_AssetOnline:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructAssetOnline<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_AssetOnline
            .get_or_init(|| {
                let def_val: Option<()> = Default::default();
                <Option<()> as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructAssetOnline<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructAssetOnline<T> {}
#[doc(hidden)]
pub struct __GetByteStructRegisteredAt<T>(
    pub self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T)>,
);
#[cfg(feature = "std")]
#[allow(non_upper_case_globals)]
static __CACHE_GET_BYTE_STRUCT_RegisteredAt:
    self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8>,
    > = self::sp_api_hidden_includes_decl_storage::hidden_include::once_cell::sync::OnceCell::new();
#[cfg(feature = "std")]
impl<T: Trait> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::DefaultByte
    for __GetByteStructRegisteredAt<T>
{
    fn default_byte(
        &self,
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::vec::Vec<u8> {
        use self::sp_api_hidden_includes_decl_storage::hidden_include::codec::Encode;
        __CACHE_GET_BYTE_STRUCT_RegisteredAt
            .get_or_init(|| {
                let def_val: T::BlockNumber = Default::default();
                <T::BlockNumber as Encode>::encode(&def_val)
            })
            .clone()
    }
}
unsafe impl<T: Trait> Send for __GetByteStructRegisteredAt<T> {}
unsafe impl<T: Trait> Sync for __GetByteStructRegisteredAt<T> {}
impl<T: Trait + 'static> Module<T> {
    #[doc(hidden)]
    pub fn storage_metadata(
    ) -> self::sp_api_hidden_includes_decl_storage::hidden_include::metadata::StorageMetadata {
        self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageMetadata { prefix : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "XAssetsRegistrar" ) , entries : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetIdsOf" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Twox64Concat , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Chain" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "Vec<AssetId>" ) , unused : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructAssetIdsOf :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " Asset id list for each Chain." ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetInfoOf" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Optional , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Twox64Concat , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetInfo" ) , unused : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructAssetInfoOf :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " Asset info of each asset." ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetOnline" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Optional , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Twox64Concat , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "()" ) , unused : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructAssetOnline :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " The map of asset to the online state." ] ) , } , self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryMetadata { name : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "RegisteredAt" ) , modifier : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryModifier :: Default , ty : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageEntryType :: Map { hasher : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: StorageHasher :: Twox64Concat , key : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "AssetId" ) , value : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( "T::BlockNumber" ) , unused : false , } , default : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DefaultByteGetter ( & __GetByteStructRegisteredAt :: < T > ( self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_std :: marker :: PhantomData ) ) ) , documentation : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: metadata :: DecodeDifferent :: Encode ( & [ " The map of asset to the block number at which the asset was registered." ] ) , } ] [ .. ] ) , }
    }
}
/// Hidden instance generated to be internally used when module is used without
/// instance.
#[doc(hidden)]
pub struct __InherentHiddenInstance;
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for __InherentHiddenInstance {
    #[inline]
    fn clone(&self) -> __InherentHiddenInstance {
        match *self {
            __InherentHiddenInstance => __InherentHiddenInstance,
        }
    }
}
impl ::core::marker::StructuralEq for __InherentHiddenInstance {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::Eq for __InherentHiddenInstance {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {}
    }
}
impl ::core::marker::StructuralPartialEq for __InherentHiddenInstance {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::cmp::PartialEq for __InherentHiddenInstance {
    #[inline]
    fn eq(&self, other: &__InherentHiddenInstance) -> bool {
        match *other {
            __InherentHiddenInstance => match *self {
                __InherentHiddenInstance => true,
            },
        }
    }
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Encode for __InherentHiddenInstance {
        fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
            &self,
            __codec_dest_edqy: &mut __CodecOutputEdqy,
        ) {
        }
    }
    impl _parity_scale_codec::EncodeLike for __InherentHiddenInstance {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl _parity_scale_codec::Decode for __InherentHiddenInstance {
        fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
            __codec_input_edqy: &mut __CodecInputEdqy,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            Ok(__InherentHiddenInstance)
        }
    }
};
impl core::fmt::Debug for __InherentHiddenInstance {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_tuple("__InherentHiddenInstance").finish()
    }
}
impl self::sp_api_hidden_includes_decl_storage::hidden_include::traits::Instance
    for __InherentHiddenInstance
{
    const PREFIX: &'static str = "XAssetsRegistrar";
}
/// Genesis config for the module, allow to build genesis storage.
#[cfg(feature = "std")]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[serde(bound(
    serialize = "Vec < (AssetId, AssetInfo, bool, bool) > : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::Serialize, "
))]
#[serde(bound(
    deserialize = "Vec < (AssetId, AssetInfo, bool, bool) > : self :: sp_api_hidden_includes_decl_storage :: hidden_include::serde::de::DeserializeOwned, "
))]
pub struct GenesisConfig {
    pub assets: Vec<(AssetId, AssetInfo, bool, bool)>,
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(rust_2018_idioms, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl _serde::Serialize for GenesisConfig
    where
        Vec<(AssetId, AssetInfo, bool, bool)>:
            self::sp_api_hidden_includes_decl_storage::hidden_include::serde::Serialize,
    {
        fn serialize<__S>(&self, __serializer: __S) -> _serde::export::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = match _serde::Serializer::serialize_struct(
                __serializer,
                "GenesisConfig",
                false as usize + 1,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            match _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "assets",
                &self.assets,
            ) {
                _serde::export::Ok(__val) => __val,
                _serde::export::Err(__err) => {
                    return _serde::export::Err(__err);
                }
            };
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(rust_2018_idioms, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for GenesisConfig
    where
        Vec<(AssetId, AssetInfo, bool, bool)>:
            self::sp_api_hidden_includes_decl_storage::hidden_include::serde::de::DeserializeOwned,
    {
        fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::export::Ok(__Field::__field0),
                        _ => _serde::export::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"field index 0 <= i < 1",
                        )),
                    }
                }
                fn visit_str<__E>(self, __value: &str) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "assets" => _serde::export::Ok(__Field::__field0),
                        _ => _serde::export::Err(_serde::de::Error::unknown_field(__value, FIELDS)),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"assets" => _serde::export::Ok(__Field::__field0),
                        _ => {
                            let __value = &_serde::export::from_utf8_lossy(__value);
                            _serde::export::Err(_serde::de::Error::unknown_field(__value, FIELDS))
                        }
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor < 'de > where Vec < ( AssetId , AssetInfo , bool , bool ) > : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned { marker : _serde :: export :: PhantomData < GenesisConfig > , lifetime : _serde :: export :: PhantomData < & 'de ( ) > , }
            impl < 'de > _serde :: de :: Visitor < 'de > for __Visitor < 'de > where Vec < ( AssetId , AssetInfo , bool , bool ) > : self :: sp_api_hidden_includes_decl_storage :: hidden_include :: serde :: de :: DeserializeOwned { type Value = GenesisConfig ; fn expecting ( & self , __formatter : & mut _serde :: export :: Formatter ) -> _serde :: export :: fmt :: Result { _serde :: export :: Formatter :: write_str ( __formatter , "struct GenesisConfig" ) } # [ inline ] fn visit_seq < __A > ( self , mut __seq : __A ) -> _serde :: export :: Result < Self :: Value , __A :: Error > where __A : _serde :: de :: SeqAccess < 'de > { let __field0 = match match _serde :: de :: SeqAccess :: next_element :: < Vec < ( AssetId , AssetInfo , bool , bool ) > > ( & mut __seq ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { _serde :: export :: Some ( __value ) => __value , _serde :: export :: None => { return _serde :: export :: Err ( _serde :: de :: Error :: invalid_length ( 0usize , & "struct GenesisConfig with 1 element" ) ) ; } } ; _serde :: export :: Ok ( GenesisConfig { assets : __field0 , } ) } # [ inline ] fn visit_map < __A > ( self , mut __map : __A ) -> _serde :: export :: Result < Self :: Value , __A :: Error > where __A : _serde :: de :: MapAccess < 'de > { let mut __field0 : _serde :: export :: Option < Vec < ( AssetId , AssetInfo , bool , bool ) > > = _serde :: export :: None ; while let _serde :: export :: Some ( __key ) = match _serde :: de :: MapAccess :: next_key :: < __Field > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } { match __key { __Field :: __field0 => { if _serde :: export :: Option :: is_some ( & __field0 ) { return _serde :: export :: Err ( < __A :: Error as _serde :: de :: Error > :: duplicate_field ( "assets" ) ) ; } __field0 = _serde :: export :: Some ( match _serde :: de :: MapAccess :: next_value :: < Vec < ( AssetId , AssetInfo , bool , bool ) > > ( & mut __map ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } ) ; } } } let __field0 = match __field0 { _serde :: export :: Some ( __field0 ) => __field0 , _serde :: export :: None => match _serde :: private :: de :: missing_field ( "assets" ) { _serde :: export :: Ok ( __val ) => __val , _serde :: export :: Err ( __err ) => { return _serde :: export :: Err ( __err ) ; } } , } ; _serde :: export :: Ok ( GenesisConfig { assets : __field0 , } ) } }
            const FIELDS: &'static [&'static str] = &["assets"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "GenesisConfig",
                FIELDS,
                __Visitor {
                    marker: _serde::export::PhantomData::<GenesisConfig>,
                    lifetime: _serde::export::PhantomData,
                },
            )
        }
    }
};
#[cfg(feature = "std")]
impl Default for GenesisConfig {
    fn default() -> Self {
        GenesisConfig {
            assets: Default::default(),
        }
    }
}
#[cfg(feature = "std")]
impl GenesisConfig {
    /// Build the storage for this module.
    pub fn build_storage<T: Trait>(
        &self,
    ) -> std::result::Result<
        self::sp_api_hidden_includes_decl_storage::hidden_include::sp_runtime::Storage,
        String,
    > {
        let mut storage = Default::default();
        self.assimilate_storage::<T>(&mut storage)?;
        Ok(storage)
    }
    /// Assimilate the storage for this module into pre-existing overlays.
    pub fn assimilate_storage<T: Trait>(
        &self,
        storage : & mut self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_runtime :: Storage,
    ) -> std::result::Result<(), String> {
        self :: sp_api_hidden_includes_decl_storage :: hidden_include :: BasicExternalities :: execute_with_storage ( storage , | | { let extra_genesis_builder : fn ( & Self ) = | config | { for ( id , asset , is_online , has_mining_rights ) in & config . assets { Module :: < T > :: register ( frame_system :: RawOrigin :: Root . into ( ) , * id , asset . clone ( ) , * is_online , * has_mining_rights ) . expect ( "asset registeration during the genesis can not fail" ) ; } } ; extra_genesis_builder ( self ) ; Ok ( ( ) ) } )
    }
}
#[cfg(feature = "std")]
impl<
        T: Trait,
        __GeneratedInstance: self::sp_api_hidden_includes_decl_storage::hidden_include::traits::Instance,
    >
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_runtime::BuildModuleGenesisStorage<
        T,
        __GeneratedInstance,
    > for GenesisConfig
{
    fn build_module_genesis_storage(
        &self,
        storage : & mut self :: sp_api_hidden_includes_decl_storage :: hidden_include :: sp_runtime :: Storage,
    ) -> std::result::Result<(), String> {
        self.assimilate_storage::<T>(storage)
    }
}
/// Asset id list for each Chain.
pub struct AssetIdsOf(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<
        Vec<AssetId>,
    > for AssetIdsOf
{
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"AssetIdsOf"
    }
}
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        Chain,
        Vec<AssetId>,
    > for AssetIdsOf
{
    type Query = Vec<AssetId>;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Twox64Concat;
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"AssetIdsOf"
    }
    fn from_optional_value_to_query(v: Option<Vec<AssetId>>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<Vec<AssetId>> {
        Some(v)
    }
}
/// Asset info of each asset.
pub struct AssetInfoOf(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<
        AssetInfo,
    > for AssetInfoOf
{
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"AssetInfoOf"
    }
}
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        AssetId,
        AssetInfo,
    > for AssetInfoOf
{
    type Query = Option<AssetInfo>;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Twox64Concat;
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"AssetInfoOf"
    }
    fn from_optional_value_to_query(v: Option<AssetInfo>) -> Self::Query {
        v.or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<AssetInfo> {
        v
    }
}
/// The map of asset to the online state.
pub struct AssetOnline(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<()>,
);
impl self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<()>
    for AssetOnline
{
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"AssetOnline"
    }
}
impl
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        AssetId,
        (),
    > for AssetOnline
{
    type Query = Option<()>;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Twox64Concat;
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"AssetOnline"
    }
    fn from_optional_value_to_query(v: Option<()>) -> Self::Query {
        v.or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<()> {
        v
    }
}
/// The map of asset to the block number at which the asset was registered.
pub struct RegisteredAt<T: Trait>(
    self::sp_api_hidden_includes_decl_storage::hidden_include::sp_std::marker::PhantomData<(T,)>,
);
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::StoragePrefixedMap<
        T::BlockNumber,
    > for RegisteredAt<T>
{
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"RegisteredAt"
    }
}
impl<T: Trait>
    self::sp_api_hidden_includes_decl_storage::hidden_include::storage::generator::StorageMap<
        AssetId,
        T::BlockNumber,
    > for RegisteredAt<T>
{
    type Query = T::BlockNumber;
    type Hasher = self::sp_api_hidden_includes_decl_storage::hidden_include::Twox64Concat;
    fn module_prefix() -> &'static [u8] {
        < __InherentHiddenInstance as self :: sp_api_hidden_includes_decl_storage :: hidden_include :: traits :: Instance > :: PREFIX . as_bytes ( )
    }
    fn storage_prefix() -> &'static [u8] {
        b"RegisteredAt"
    }
    fn from_optional_value_to_query(v: Option<T::BlockNumber>) -> Self::Query {
        v.unwrap_or_else(|| Default::default())
    }
    fn from_query_to_optional_value(v: Self::Query) -> Option<T::BlockNumber> {
        Some(v)
    }
}
pub struct Module<T: Trait>(::frame_support::sp_std::marker::PhantomData<(T,)>);
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::clone::Clone + Trait> ::core::clone::Clone for Module<T> {
    #[inline]
    fn clone(&self) -> Module<T> {
        match *self {
            Module(ref __self_0_0) => Module(::core::clone::Clone::clone(&(*__self_0_0))),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::marker::Copy + Trait> ::core::marker::Copy for Module<T> {}
impl<T: Trait> ::core::marker::StructuralPartialEq for Module<T> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::cmp::PartialEq + Trait> ::core::cmp::PartialEq for Module<T> {
    #[inline]
    fn eq(&self, other: &Module<T>) -> bool {
        match *other {
            Module(ref __self_1_0) => match *self {
                Module(ref __self_0_0) => (*__self_0_0) == (*__self_1_0),
            },
        }
    }
    #[inline]
    fn ne(&self, other: &Module<T>) -> bool {
        match *other {
            Module(ref __self_1_0) => match *self {
                Module(ref __self_0_0) => (*__self_0_0) != (*__self_1_0),
            },
        }
    }
}
impl<T: Trait> ::core::marker::StructuralEq for Module<T> {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<T: ::core::cmp::Eq + Trait> ::core::cmp::Eq for Module<T> {
    #[inline]
    #[doc(hidden)]
    fn assert_receiver_is_total_eq(&self) -> () {
        {
            let _: ::core::cmp::AssertParamIsEq<
                ::frame_support::sp_std::marker::PhantomData<(T,)>,
            >;
        }
    }
}
impl<T: Trait> core::fmt::Debug for Module<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_tuple("Module").field(&self.0).finish()
    }
}
impl<T: Trait> ::frame_support::traits::OnInitialize<T::BlockNumber> for Module<T> {}
impl<T: Trait> ::frame_support::traits::OnRuntimeUpgrade for Module<T> {}
impl<T: Trait> ::frame_support::traits::OnFinalize<T::BlockNumber> for Module<T> {}
impl<T: Trait> ::frame_support::traits::OffchainWorker<T::BlockNumber> for Module<T> {}
impl<T: Trait> Module<T> {
    /// Deposits an event using `frame_system::Module::deposit_event`.
    fn deposit_event(event: impl Into<<T as Trait>::Event>) {
        <frame_system::Module<T>>::deposit_event(event.into())
    }
}
#[cfg(feature = "std")]
impl<T: Trait> ::frame_support::traits::IntegrityTest for Module<T> {}
/// Can also be called using [`Call`].
///
/// [`Call`]: enum.Call.html
impl<T: Trait> Module<T> {
    /// Register a new foreign asset.
    ///
    /// This is a root-only operation.
    ///
    /// NOTE: Calling this function will bypass origin filters.
    pub fn register(
        origin: T::Origin,
        asset_id: AssetId,
        asset: AssetInfo,
        is_online: bool,
        has_mining_rights: bool,
    ) -> DispatchResult {
        let __tracing_span__ = {
            {
                if ::sp_tracing::tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && ::sp_tracing::tracing::Level::TRACE
                        <= ::tracing::level_filters::LevelFilter::current()
                {
                    use ::tracing::__macro_support::*;
                    let callsite = {
                        use ::tracing::__macro_support::MacroCallsite;
                        static META: ::tracing::Metadata<'static> = {
                            ::tracing_core::metadata::Metadata::new(
                                "register",
                                "xpallet_assets_registrar",
                                ::sp_tracing::tracing::Level::TRACE,
                                Some("xpallets/assets-registrar/src/lib.rs"),
                                Some(129u32),
                                Some("xpallet_assets_registrar"),
                                ::tracing_core::field::FieldSet::new(
                                    &[],
                                    ::tracing_core::callsite::Identifier(&CALLSITE),
                                ),
                                ::tracing::metadata::Kind::SPAN,
                            )
                        };
                        static CALLSITE: MacroCallsite = MacroCallsite::new(&META);
                        CALLSITE.register();
                        &CALLSITE
                    };
                    if callsite.is_enabled() {
                        let meta = callsite.metadata();
                        ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                    } else {
                        ::tracing::Span::none()
                    }
                } else {
                    ::tracing::Span::none()
                }
            }
        };
        let __tracing_guard__ = { __tracing_span__.enter() };
        ensure_root(origin)?;
        asset.is_valid::<T>()?;
        {
            if !!Self::asset_is_exists(&asset_id) {
                {
                    return Err(Error::<T>::AssetAlreadyExists.into());
                };
            }
        };
        {
            let lvl = ::log::Level::Info;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api_log(
                    ::core::fmt::Arguments::new_v1(
                        &[
                            "[register_asset]|id:",
                            "|",
                            "|is_online:",
                            "|has_mining_rights:",
                        ],
                        &match (&asset_id, &asset, &is_online, &has_mining_rights) {
                            (arg0, arg1, arg2, arg3) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg2, ::core::fmt::Display::fmt),
                                ::core::fmt::ArgumentV1::new(arg3, ::core::fmt::Display::fmt),
                            ],
                        },
                    ),
                    lvl,
                    &(
                        &{
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["runtime::"],
                                &match (&"xpallet_assets_registrar",) {
                                    (arg0,) => [::core::fmt::ArgumentV1::new(
                                        arg0,
                                        ::core::fmt::Display::fmt,
                                    )],
                                },
                            ));
                            res
                        },
                        "xpallet_assets_registrar",
                        "xpallets/assets-registrar/src/lib.rs",
                        151u32,
                    ),
                );
            }
        };
        Self::apply_register(asset_id, asset)?;
        T::RegistrarHandler::on_register(&asset_id, has_mining_rights)?;
        Self::deposit_event(Event::Register(asset_id, has_mining_rights));
        if !is_online {
            let _ = Self::deregister(frame_system::RawOrigin::Root.into(), asset_id);
        }
        Ok(())
    }
    /// Deregister an asset with given `id`.
    ///
    /// This asset will be marked as invalid.
    ///
    /// This is a root-only operation.
    ///
    /// NOTE: Calling this function will bypass origin filters.
    pub fn deregister(origin: T::Origin, id: AssetId) -> DispatchResult {
        let __tracing_span__ = {
            {
                if ::sp_tracing::tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && ::sp_tracing::tracing::Level::TRACE
                        <= ::tracing::level_filters::LevelFilter::current()
                {
                    use ::tracing::__macro_support::*;
                    let callsite = {
                        use ::tracing::__macro_support::MacroCallsite;
                        static META: ::tracing::Metadata<'static> = {
                            ::tracing_core::metadata::Metadata::new(
                                "deregister",
                                "xpallet_assets_registrar",
                                ::sp_tracing::tracing::Level::TRACE,
                                Some("xpallets/assets-registrar/src/lib.rs"),
                                Some(129u32),
                                Some("xpallet_assets_registrar"),
                                ::tracing_core::field::FieldSet::new(
                                    &[],
                                    ::tracing_core::callsite::Identifier(&CALLSITE),
                                ),
                                ::tracing::metadata::Kind::SPAN,
                            )
                        };
                        static CALLSITE: MacroCallsite = MacroCallsite::new(&META);
                        CALLSITE.register();
                        &CALLSITE
                    };
                    if callsite.is_enabled() {
                        let meta = callsite.metadata();
                        ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                    } else {
                        ::tracing::Span::none()
                    }
                } else {
                    ::tracing::Span::none()
                }
            }
        };
        let __tracing_guard__ = { __tracing_span__.enter() };
        ensure_root(origin)?;
        {
            if !Self::asset_is_valid(&id) {
                {
                    return Err(Error::<T>::AssetIsInvalid.into());
                };
            }
        };
        AssetOnline::remove(id);
        T::RegistrarHandler::on_deregister(&id)?;
        Self::deposit_event(Event::Deregister(id));
        Ok(())
    }
    /// Recover a deregister asset to the valid state.
    ///
    /// `RegistrarHandler::on_register()` will be triggered again during the recover process.
    ///
    /// This is a root-only operation.
    ///
    /// NOTE: Calling this function will bypass origin filters.
    pub fn recover(origin: T::Origin, id: AssetId, has_mining_rights: bool) -> DispatchResult {
        let __tracing_span__ = {
            {
                if ::sp_tracing::tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && ::sp_tracing::tracing::Level::TRACE
                        <= ::tracing::level_filters::LevelFilter::current()
                {
                    use ::tracing::__macro_support::*;
                    let callsite = {
                        use ::tracing::__macro_support::MacroCallsite;
                        static META: ::tracing::Metadata<'static> = {
                            ::tracing_core::metadata::Metadata::new(
                                "recover",
                                "xpallet_assets_registrar",
                                ::sp_tracing::tracing::Level::TRACE,
                                Some("xpallets/assets-registrar/src/lib.rs"),
                                Some(129u32),
                                Some("xpallet_assets_registrar"),
                                ::tracing_core::field::FieldSet::new(
                                    &[],
                                    ::tracing_core::callsite::Identifier(&CALLSITE),
                                ),
                                ::tracing::metadata::Kind::SPAN,
                            )
                        };
                        static CALLSITE: MacroCallsite = MacroCallsite::new(&META);
                        CALLSITE.register();
                        &CALLSITE
                    };
                    if callsite.is_enabled() {
                        let meta = callsite.metadata();
                        ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                    } else {
                        ::tracing::Span::none()
                    }
                } else {
                    ::tracing::Span::none()
                }
            }
        };
        let __tracing_guard__ = { __tracing_span__.enter() };
        ensure_root(origin)?;
        {
            if !Self::asset_is_exists(&id) {
                {
                    return Err(Error::<T>::AssetIsNotExist.into());
                };
            }
        };
        {
            if !!Self::asset_is_valid(&id) {
                {
                    return Err(Error::<T>::AssetAlreadyValid.into());
                };
            }
        };
        AssetOnline::insert(id, ());
        T::RegistrarHandler::on_register(&id, has_mining_rights)?;
        Self::deposit_event(Event::Recover(id, has_mining_rights));
        Ok(())
    }
    /// Update the asset info, all the new fields are optional.
    ///
    /// This is a root-only operation.
    ///
    /// NOTE: Calling this function will bypass origin filters.
    pub fn update_asset_info(
        origin: T::Origin,
        id: AssetId,
        token: Option<Token>,
        token_name: Option<Token>,
        desc: Option<Desc>,
    ) -> DispatchResult {
        let __tracing_span__ = {
            {
                if ::sp_tracing::tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && ::sp_tracing::tracing::Level::TRACE
                        <= ::tracing::level_filters::LevelFilter::current()
                {
                    use ::tracing::__macro_support::*;
                    let callsite = {
                        use ::tracing::__macro_support::MacroCallsite;
                        static META: ::tracing::Metadata<'static> = {
                            ::tracing_core::metadata::Metadata::new(
                                "update_asset_info",
                                "xpallet_assets_registrar",
                                ::sp_tracing::tracing::Level::TRACE,
                                Some("xpallets/assets-registrar/src/lib.rs"),
                                Some(129u32),
                                Some("xpallet_assets_registrar"),
                                ::tracing_core::field::FieldSet::new(
                                    &[],
                                    ::tracing_core::callsite::Identifier(&CALLSITE),
                                ),
                                ::tracing::metadata::Kind::SPAN,
                            )
                        };
                        static CALLSITE: MacroCallsite = MacroCallsite::new(&META);
                        CALLSITE.register();
                        &CALLSITE
                    };
                    if callsite.is_enabled() {
                        let meta = callsite.metadata();
                        ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                    } else {
                        ::tracing::Span::none()
                    }
                } else {
                    ::tracing::Span::none()
                }
            }
        };
        let __tracing_guard__ = { __tracing_span__.enter() };
        ensure_root(origin)?;
        let mut info = Self::asset_info_of(&id).ok_or(Error::<T>::AssetIsNotExist)?;
        if let Some(t) = token {
            info.set_token(t)
        }
        if let Some(name) = token_name {
            info.set_token_name(name);
        }
        if let Some(desc) = desc {
            info.set_desc(desc);
        }
        AssetInfoOf::insert(id, info);
        Ok(())
    }
}
/// Dispatchable calls.
///
/// Each variant of this enum maps to a dispatchable function from the associated module.
pub enum Call<T: Trait> {
    #[doc(hidden)]
    #[codec(skip)]
    __PhantomItem(
        ::frame_support::sp_std::marker::PhantomData<(T,)>,
        ::frame_support::Never,
    ),
    #[allow(non_camel_case_types)]
    /// Register a new foreign asset.
    ///
    /// This is a root-only operation.
    register(#[codec(compact)] AssetId, AssetInfo, bool, bool),
    #[allow(non_camel_case_types)]
    /// Deregister an asset with given `id`.
    ///
    /// This asset will be marked as invalid.
    ///
    /// This is a root-only operation.
    deregister(#[codec(compact)] AssetId),
    #[allow(non_camel_case_types)]
    /// Recover a deregister asset to the valid state.
    ///
    /// `RegistrarHandler::on_register()` will be triggered again during the recover process.
    ///
    /// This is a root-only operation.
    recover(#[codec(compact)] AssetId, bool),
    #[allow(non_camel_case_types)]
    /// Update the asset info, all the new fields are optional.
    ///
    /// This is a root-only operation.
    update_asset_info(
        #[codec(compact)] AssetId,
        Option<Token>,
        Option<Token>,
        Option<Desc>,
    ),
}
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<T: Trait> _parity_scale_codec::Encode for Call<T> {
        fn encode_to<__CodecOutputEdqy: _parity_scale_codec::Output>(
            &self,
            __codec_dest_edqy: &mut __CodecOutputEdqy,
        ) {
            match *self {
                Call::register(ref aa, ref ba, ref ca, ref da) => {
                    __codec_dest_edqy.push_byte(0usize as u8);
                    {
                        __codec_dest_edqy . push ( & < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: EncodeAsRef < '_ , AssetId > > :: from ( aa ) ) ;
                    }
                    __codec_dest_edqy.push(ba);
                    __codec_dest_edqy.push(ca);
                    __codec_dest_edqy.push(da);
                }
                Call::deregister(ref aa) => {
                    __codec_dest_edqy.push_byte(1usize as u8);
                    {
                        __codec_dest_edqy . push ( & < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: EncodeAsRef < '_ , AssetId > > :: from ( aa ) ) ;
                    }
                }
                Call::recover(ref aa, ref ba) => {
                    __codec_dest_edqy.push_byte(2usize as u8);
                    {
                        __codec_dest_edqy . push ( & < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: EncodeAsRef < '_ , AssetId > > :: from ( aa ) ) ;
                    }
                    __codec_dest_edqy.push(ba);
                }
                Call::update_asset_info(ref aa, ref ba, ref ca, ref da) => {
                    __codec_dest_edqy.push_byte(3usize as u8);
                    {
                        __codec_dest_edqy . push ( & < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: EncodeAsRef < '_ , AssetId > > :: from ( aa ) ) ;
                    }
                    __codec_dest_edqy.push(ba);
                    __codec_dest_edqy.push(ca);
                    __codec_dest_edqy.push(da);
                }
                _ => (),
            }
        }
    }
    impl<T: Trait> _parity_scale_codec::EncodeLike for Call<T> {}
};
const _: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate codec as _parity_scale_codec;
    impl<T: Trait> _parity_scale_codec::Decode for Call<T> {
        fn decode<__CodecInputEdqy: _parity_scale_codec::Input>(
            __codec_input_edqy: &mut __CodecInputEdqy,
        ) -> core::result::Result<Self, _parity_scale_codec::Error> {
            match __codec_input_edqy.read_byte()? {
                __codec_x_edqy if __codec_x_edqy == 0usize as u8 => Ok(Call::register(
                    {
                        let __codec_res_edqy = < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: Decode > :: decode ( __codec_input_edqy ) ;
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Call :: register.0".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy.into(),
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Call :: register.1".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Call :: register.2".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Call :: register.3".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                )),
                __codec_x_edqy if __codec_x_edqy == 1usize as u8 => Ok(Call::deregister({
                    let __codec_res_edqy = < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: Decode > :: decode ( __codec_input_edqy ) ;
                    match __codec_res_edqy {
                        Err(_) => return Err("Error decoding field Call :: deregister.0".into()),
                        Ok(__codec_res_edqy) => __codec_res_edqy.into(),
                    }
                })),
                __codec_x_edqy if __codec_x_edqy == 2usize as u8 => Ok(Call::recover(
                    {
                        let __codec_res_edqy = < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: Decode > :: decode ( __codec_input_edqy ) ;
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Call :: recover.0".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy.into(),
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => return Err("Error decoding field Call :: recover.1".into()),
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                )),
                __codec_x_edqy if __codec_x_edqy == 3usize as u8 => Ok(Call::update_asset_info(
                    {
                        let __codec_res_edqy = < < AssetId as _parity_scale_codec :: HasCompact > :: Type as _parity_scale_codec :: Decode > :: decode ( __codec_input_edqy ) ;
                        match __codec_res_edqy {
                            Err(_) => {
                                return Err(
                                    "Error decoding field Call :: update_asset_info.0".into()
                                )
                            }
                            Ok(__codec_res_edqy) => __codec_res_edqy.into(),
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => {
                                return Err(
                                    "Error decoding field Call :: update_asset_info.1".into()
                                )
                            }
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => {
                                return Err(
                                    "Error decoding field Call :: update_asset_info.2".into()
                                )
                            }
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                    {
                        let __codec_res_edqy =
                            _parity_scale_codec::Decode::decode(__codec_input_edqy);
                        match __codec_res_edqy {
                            Err(_) => {
                                return Err(
                                    "Error decoding field Call :: update_asset_info.3".into()
                                )
                            }
                            Ok(__codec_res_edqy) => __codec_res_edqy,
                        }
                    },
                )),
                _ => Err("No such variant in enum Call".into()),
            }
        }
    }
};
impl<T: Trait> ::frame_support::dispatch::GetDispatchInfo for Call<T> {
    fn get_dispatch_info(&self) -> ::frame_support::dispatch::DispatchInfo {
        match *self {
            Call::register(ref asset_id, ref asset, ref is_online, ref has_mining_rights) => {
                let base_weight = T::WeightInfo::register();
                let weight = <dyn ::frame_support::dispatch::WeighData<(
                    &AssetId,
                    &AssetInfo,
                    &bool,
                    &bool,
                )>>::weigh_data(
                    &base_weight,
                    (asset_id, asset, is_online, has_mining_rights),
                );
                let class = <dyn ::frame_support::dispatch::ClassifyDispatch<(
                    &AssetId,
                    &AssetInfo,
                    &bool,
                    &bool,
                )>>::classify_dispatch(
                    &base_weight,
                    (asset_id, asset, is_online, has_mining_rights),
                );
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(
                    &AssetId,
                    &AssetInfo,
                    &bool,
                    &bool,
                )>>::pays_fee(
                    &base_weight,
                    (asset_id, asset, is_online, has_mining_rights),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::deregister(ref id) => {
                let base_weight = T::WeightInfo::deregister();
                let weight = <dyn ::frame_support::dispatch::WeighData<(&AssetId,)>>::weigh_data(
                    &base_weight,
                    (id,),
                );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & AssetId , ) > > :: classify_dispatch ( & base_weight , ( id , ) ) ;
                let pays_fee = <dyn ::frame_support::dispatch::PaysFee<(&AssetId,)>>::pays_fee(
                    &base_weight,
                    (id,),
                );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::recover(ref id, ref has_mining_rights) => {
                let base_weight = T::WeightInfo::recover();
                let weight =
                    <dyn ::frame_support::dispatch::WeighData<(&AssetId, &bool)>>::weigh_data(
                        &base_weight,
                        (id, has_mining_rights),
                    );
                let class = < dyn :: frame_support :: dispatch :: ClassifyDispatch < ( & AssetId , & bool ) > > :: classify_dispatch ( & base_weight , ( id , has_mining_rights ) ) ;
                let pays_fee =
                    <dyn ::frame_support::dispatch::PaysFee<(&AssetId, &bool)>>::pays_fee(
                        &base_weight,
                        (id, has_mining_rights),
                    );
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::update_asset_info(ref id, ref token, ref token_name, ref desc) => {
                let base_weight = T::WeightInfo::update_asset_info();
                let weight =
                    <dyn ::frame_support::dispatch::WeighData<(
                        &AssetId,
                        &Option<Token>,
                        &Option<Token>,
                        &Option<Desc>,
                    )>>::weigh_data(&base_weight, (id, token, token_name, desc));
                let class = <dyn ::frame_support::dispatch::ClassifyDispatch<(
                    &AssetId,
                    &Option<Token>,
                    &Option<Token>,
                    &Option<Desc>,
                )>>::classify_dispatch(
                    &base_weight, (id, token, token_name, desc)
                );
                let pays_fee =
                    <dyn ::frame_support::dispatch::PaysFee<(
                        &AssetId,
                        &Option<Token>,
                        &Option<Token>,
                        &Option<Desc>,
                    )>>::pays_fee(&base_weight, (id, token, token_name, desc));
                ::frame_support::dispatch::DispatchInfo {
                    weight,
                    class,
                    pays_fee,
                }
            }
            Call::__PhantomItem(_, _) => {
                ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                    &["internal error: entered unreachable code: "],
                    &match (&"__PhantomItem should never be used.",) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ))
            }
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::GetCallName for Call<T> {
    fn get_call_name(&self) -> &'static str {
        match *self {
            Call::register(ref asset_id, ref asset, ref is_online, ref has_mining_rights) => {
                let _ = (asset_id, asset, is_online, has_mining_rights);
                "register"
            }
            Call::deregister(ref id) => {
                let _ = (id);
                "deregister"
            }
            Call::recover(ref id, ref has_mining_rights) => {
                let _ = (id, has_mining_rights);
                "recover"
            }
            Call::update_asset_info(ref id, ref token, ref token_name, ref desc) => {
                let _ = (id, token, token_name, desc);
                "update_asset_info"
            }
            Call::__PhantomItem(_, _) => {
                ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                    &["internal error: entered unreachable code: "],
                    &match (&"__PhantomItem should never be used.",) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ))
            }
        }
    }
    fn get_call_names() -> &'static [&'static str] {
        &["register", "deregister", "recover", "update_asset_info"]
    }
}
impl<T: Trait> ::frame_support::dispatch::Clone for Call<T> {
    fn clone(&self) -> Self {
        match *self {
            Call::register(ref asset_id, ref asset, ref is_online, ref has_mining_rights) => {
                Call::register(
                    (*asset_id).clone(),
                    (*asset).clone(),
                    (*is_online).clone(),
                    (*has_mining_rights).clone(),
                )
            }
            Call::deregister(ref id) => Call::deregister((*id).clone()),
            Call::recover(ref id, ref has_mining_rights) => {
                Call::recover((*id).clone(), (*has_mining_rights).clone())
            }
            Call::update_asset_info(ref id, ref token, ref token_name, ref desc) => {
                Call::update_asset_info(
                    (*id).clone(),
                    (*token).clone(),
                    (*token_name).clone(),
                    (*desc).clone(),
                )
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::PartialEq for Call<T> {
    fn eq(&self, _other: &Self) -> bool {
        match *self {
            Call::register(ref asset_id, ref asset, ref is_online, ref has_mining_rights) => {
                let self_params = (asset_id, asset, is_online, has_mining_rights);
                if let Call::register(
                    ref asset_id,
                    ref asset,
                    ref is_online,
                    ref has_mining_rights,
                ) = *_other
                {
                    self_params == (asset_id, asset, is_online, has_mining_rights)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::deregister(ref id) => {
                let self_params = (id,);
                if let Call::deregister(ref id) = *_other {
                    self_params == (id,)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::recover(ref id, ref has_mining_rights) => {
                let self_params = (id, has_mining_rights);
                if let Call::recover(ref id, ref has_mining_rights) = *_other {
                    self_params == (id, has_mining_rights)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            Call::update_asset_info(ref id, ref token, ref token_name, ref desc) => {
                let self_params = (id, token, token_name, desc);
                if let Call::update_asset_info(ref id, ref token, ref token_name, ref desc) =
                    *_other
                {
                    self_params == (id, token, token_name, desc)
                } else {
                    match *_other {
                        Call::__PhantomItem(_, _) => {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                        _ => false,
                    }
                }
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::Eq for Call<T> {}
impl<T: Trait> ::frame_support::dispatch::fmt::Debug for Call<T> {
    fn fmt(
        &self,
        _f: &mut ::frame_support::dispatch::fmt::Formatter,
    ) -> ::frame_support::dispatch::result::Result<(), ::frame_support::dispatch::fmt::Error> {
        match *self {
            Call::register(ref asset_id, ref asset, ref is_online, ref has_mining_rights) => _f
                .write_fmt(::core::fmt::Arguments::new_v1(
                    &["", ""],
                    &match (
                        &"register",
                        &(
                            asset_id.clone(),
                            asset.clone(),
                            is_online.clone(),
                            has_mining_rights.clone(),
                        ),
                    ) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                        ],
                    },
                )),
            Call::deregister(ref id) => _f.write_fmt(::core::fmt::Arguments::new_v1(
                &["", ""],
                &match (&"deregister", &(id.clone(),)) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                    ],
                },
            )),
            Call::recover(ref id, ref has_mining_rights) => {
                _f.write_fmt(::core::fmt::Arguments::new_v1(
                    &["", ""],
                    &match (&"recover", &(id.clone(), has_mining_rights.clone())) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                        ],
                    },
                ))
            }
            Call::update_asset_info(ref id, ref token, ref token_name, ref desc) => {
                _f.write_fmt(::core::fmt::Arguments::new_v1(
                    &["", ""],
                    &match (
                        &"update_asset_info",
                        &(id.clone(), token.clone(), token_name.clone(), desc.clone()),
                    ) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Debug::fmt),
                        ],
                    },
                ))
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
impl<T: Trait> ::frame_support::traits::UnfilteredDispatchable for Call<T> {
    type Origin = T::Origin;
    fn dispatch_bypass_filter(
        self,
        _origin: Self::Origin,
    ) -> ::frame_support::dispatch::DispatchResultWithPostInfo {
        match self {
            Call::register(asset_id, asset, is_online, has_mining_rights) => {
                <Module<T>>::register(_origin, asset_id, asset, is_online, has_mining_rights)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            Call::deregister(id) => <Module<T>>::deregister(_origin, id)
                .map(Into::into)
                .map_err(Into::into),
            Call::recover(id, has_mining_rights) => {
                <Module<T>>::recover(_origin, id, has_mining_rights)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            Call::update_asset_info(id, token, token_name, desc) => {
                <Module<T>>::update_asset_info(_origin, id, token, token_name, desc)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            Call::__PhantomItem(_, _) => {
                ::std::rt::begin_panic_fmt(&::core::fmt::Arguments::new_v1(
                    &["internal error: entered unreachable code: "],
                    &match (&"__PhantomItem should never be used.",) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ))
            }
        }
    }
}
impl<T: Trait> ::frame_support::dispatch::Callable<T> for Module<T> {
    type Call = Call<T>;
}
impl<T: Trait> Module<T> {
    #[doc(hidden)]
    pub fn call_functions() -> &'static [::frame_support::dispatch::FunctionMetadata] {
        &[
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("register"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("asset_id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Compact<AssetId>"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("asset"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("AssetInfo"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("is_online"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("bool"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode(
                            "has_mining_rights",
                        ),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("bool"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    r" Register a new foreign asset.",
                    r"",
                    r" This is a root-only operation.",
                ]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("deregister"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Compact<AssetId>"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    r" Deregister an asset with given `id`.",
                    r"",
                    r" This asset will be marked as invalid.",
                    r"",
                    r" This is a root-only operation.",
                ]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("recover"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Compact<AssetId>"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode(
                            "has_mining_rights",
                        ),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("bool"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    r" Recover a deregister asset to the valid state.",
                    r"",
                    r" `RegistrarHandler::on_register()` will be triggered again during the recover process.",
                    r"",
                    r" This is a root-only operation.",
                ]),
            },
            ::frame_support::dispatch::FunctionMetadata {
                name: ::frame_support::dispatch::DecodeDifferent::Encode("update_asset_info"),
                arguments: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("id"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Compact<AssetId>"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("token"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Option<Token>"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("token_name"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Option<Token>"),
                    },
                    ::frame_support::dispatch::FunctionArgumentMetadata {
                        name: ::frame_support::dispatch::DecodeDifferent::Encode("desc"),
                        ty: ::frame_support::dispatch::DecodeDifferent::Encode("Option<Desc>"),
                    },
                ]),
                documentation: ::frame_support::dispatch::DecodeDifferent::Encode(&[
                    r" Update the asset info, all the new fields are optional.",
                    r"",
                    r" This is a root-only operation.",
                ]),
            },
        ]
    }
}
impl<T: 'static + Trait> Module<T> {
    #[doc(hidden)]
    pub fn module_constants_metadata(
    ) -> &'static [::frame_support::dispatch::ModuleConstantMetadata] {
        &[]
    }
}
impl<T: Trait> ::frame_support::dispatch::ModuleErrorMetadata for Module<T> {
    fn metadata() -> &'static [::frame_support::dispatch::ErrorMetadata] {
        <Error<T> as ::frame_support::dispatch::ModuleErrorMetadata>::metadata()
    }
}
impl<T: Trait> Module<T> {
    /// Returns an iterator of all the asset ids of all chains so far.
    #[inline]
    pub fn asset_ids() -> impl Iterator<Item = AssetId> {
        Chain::iter().map(Self::asset_ids_of).flatten()
    }
    /// Returns an iterator of all the valid asset ids of all chains so far.
    #[inline]
    pub fn valid_asset_ids() -> impl Iterator<Item = AssetId> {
        Self::asset_ids().filter(Self::asset_is_valid)
    }
    /// Returns an iterator of tuple (AssetId, AssetInfo) of all assets.
    #[inline]
    pub fn asset_infos() -> impl Iterator<Item = (AssetId, AssetInfo)> {
        AssetInfoOf::iter()
    }
    /// Returns an iterator of tuple (AssetId, AssetInfo) of all valid assets.
    #[inline]
    pub fn valid_asset_infos() -> impl Iterator<Item = (AssetId, AssetInfo)> {
        Self::asset_infos().filter(|(id, _)| Self::asset_is_valid(id))
    }
    /// Returns the chain of given asset `asset_id`.
    pub fn chain_of(asset_id: &AssetId) -> result::Result<Chain, DispatchError> {
        Self::asset_info_of(asset_id)
            .map(|info| info.chain())
            .ok_or_else(|| Error::<T>::AssetIsNotExist.into())
    }
    /// Returns the asset info of given `id`.
    pub fn get_asset_info(id: &AssetId) -> result::Result<AssetInfo, DispatchError> {
        if let Some(asset) = Self::asset_info_of(id) {
            if Self::asset_is_valid(id) {
                Ok(asset)
            } else {
                Err(Error::<T>::AssetIsInvalid.into())
            }
        } else {
            Err(Error::<T>::AssetIsNotExist.into())
        }
    }
    /// Returns true if the given `asset_id` is an online asset.
    pub fn is_online(asset_id: &AssetId) -> bool {
        Self::asset_online(asset_id).is_some()
    }
    /// Returns true if the asset info record of given `asset_id` exists.
    pub fn asset_is_exists(asset_id: &AssetId) -> bool {
        Self::asset_info_of(asset_id).is_some()
    }
    /// Returns true if the asset of given `asset_id` is still online.
    pub fn asset_is_valid(asset_id: &AssetId) -> bool {
        Self::is_online(asset_id)
    }
    /// Helper function for checking the asset's existence.
    pub fn ensure_asset_is_exists(id: &AssetId) -> DispatchResult {
        {
            if !Self::asset_is_exists(id) {
                {
                    return Err(Error::<T>::AssetIsNotExist.into());
                };
            }
        };
        Ok(())
    }
    /// Helper function for checking the asset's validity.
    pub fn ensure_asset_is_valid(id: &AssetId) -> DispatchResult {
        {
            if !Self::asset_is_valid(id) {
                {
                    return Err(Error::<T>::AssetIsInvalid.into());
                };
            }
        };
        Ok(())
    }
    /// Actually register an asset.
    fn apply_register(id: AssetId, asset: AssetInfo) -> DispatchResult {
        let chain = asset.chain();
        AssetIdsOf::mutate(chain, |ids| {
            if !ids.contains(&id) {
                ids.push(id);
            }
        });
        AssetInfoOf::insert(&id, asset);
        AssetOnline::insert(&id, ());
        RegisteredAt::<T>::insert(&id, frame_system::Module::<T>::block_number());
        Ok(())
    }
}
