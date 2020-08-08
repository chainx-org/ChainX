#![feature(prelude_import)]
//! RPC interface for the transaction payment module.
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
use chainx_primitives::AssetId;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::collections::btree_map::BTreeMap;
use std::sync::Arc;
use xpallet_mining_asset::{MiningAssetInfo, RpcMinerLedger};
use xpallet_mining_asset_rpc_runtime_api::XMiningAssetApi as XMiningAssetRuntimeApi;
use xpallet_support::RpcBalance;
mod rpc_impl_XMiningAssetApi {
    use super::*;
    use jsonrpc_core as _jsonrpc_core;
    /// The generated client module.
    pub mod gen_client {
        use super::*;
        use _jsonrpc_core::futures::prelude::*;
        use _jsonrpc_core::futures::sync::{mpsc, oneshot};
        use _jsonrpc_core::serde_json::{self, Value};
        use _jsonrpc_core::{
            Call, Error, ErrorCode, Id, MethodCall, Params, Request, Response, Version,
        };
        use _jsonrpc_core_client::{
            RpcChannel, RpcError, RpcFuture, TypedClient, TypedSubscriptionStream,
        };
        use jsonrpc_core_client as _jsonrpc_core_client;
        /// The Client.
        pub struct Client<BlockHash, AccountId, RpcBalance, BlockNumber> {
            inner: TypedClient,
            _0: std::marker::PhantomData<BlockHash>,
            _1: std::marker::PhantomData<AccountId>,
            _2: std::marker::PhantomData<RpcBalance>,
            _3: std::marker::PhantomData<BlockNumber>,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl<
                BlockHash: ::core::clone::Clone,
                AccountId: ::core::clone::Clone,
                RpcBalance: ::core::clone::Clone,
                BlockNumber: ::core::clone::Clone,
            > ::core::clone::Clone for Client<BlockHash, AccountId, RpcBalance, BlockNumber>
        {
            #[inline]
            fn clone(&self) -> Client<BlockHash, AccountId, RpcBalance, BlockNumber> {
                match *self {
                    Client {
                        inner: ref __self_0_0,
                        _0: ref __self_0_1,
                        _1: ref __self_0_2,
                        _2: ref __self_0_3,
                        _3: ref __self_0_4,
                    } => Client {
                        inner: ::core::clone::Clone::clone(&(*__self_0_0)),
                        _0: ::core::clone::Clone::clone(&(*__self_0_1)),
                        _1: ::core::clone::Clone::clone(&(*__self_0_2)),
                        _2: ::core::clone::Clone::clone(&(*__self_0_3)),
                        _3: ::core::clone::Clone::clone(&(*__self_0_4)),
                    },
                }
            }
        }
        impl<BlockHash, AccountId, RpcBalance, BlockNumber>
            Client<BlockHash, AccountId, RpcBalance, BlockNumber>
        where
            BlockHash: Send + Sync + 'static + _jsonrpc_core::serde::Serialize,
            AccountId: Send
                + Sync
                + 'static
                + _jsonrpc_core::serde::de::DeserializeOwned
                + _jsonrpc_core::serde::Serialize,
            RpcBalance: Send + Sync + 'static + _jsonrpc_core::serde::de::DeserializeOwned,
            BlockNumber: Send + Sync + 'static + _jsonrpc_core::serde::de::DeserializeOwned,
        {
            /// Creates a new `Client`.
            pub fn new(sender: RpcChannel) -> Self {
                Client {
                    inner: sender.into(),
                    _0: std::marker::PhantomData,
                    _1: std::marker::PhantomData,
                    _2: std::marker::PhantomData,
                    _3: std::marker::PhantomData,
                }
            }
            /// Get overall information about all mining assets.
            pub fn mining_assets(
                &self,
                at: Option<BlockHash>,
            ) -> impl Future<
                Item = Vec<MiningAssetInfo<AccountId, RpcBalance, BlockNumber>>,
                Error = RpcError,
            > {
                let args = (at,);
                self.inner.call_method(
                    "xminingasset_getMiningAssets",
                    "Vec < MiningAssetInfo < AccountId, RpcBalance, BlockNumber > >",
                    args,
                )
            }
            /// Get the asset mining dividends info given the asset miner AccountId.
            pub fn mining_dividend(
                &self,
                who: AccountId,
                at: Option<BlockHash>,
            ) -> impl Future<Item = BTreeMap<AssetId, RpcBalance>, Error = RpcError> {
                let args = (who, at);
                self.inner.call_method(
                    "xminingasset_getDividendByAccount",
                    "BTreeMap < AssetId, RpcBalance >",
                    args,
                )
            }
            /// Get the mining ledger details given the asset miner AccountId.
            pub fn miner_ledger(
                &self,
                who: AccountId,
                at: Option<BlockHash>,
            ) -> impl Future<Item = BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>, Error = RpcError>
            {
                let args = (who, at);
                self.inner.call_method(
                    "xminingasset_getMinerLedgerByAccount",
                    "BTreeMap < AssetId, RpcMinerLedger < BlockNumber > >",
                    args,
                )
            }
        }
        impl<BlockHash, AccountId, RpcBalance, BlockNumber> From<RpcChannel>
            for Client<BlockHash, AccountId, RpcBalance, BlockNumber>
        where
            BlockHash: Send + Sync + 'static + _jsonrpc_core::serde::Serialize,
            AccountId: Send
                + Sync
                + 'static
                + _jsonrpc_core::serde::de::DeserializeOwned
                + _jsonrpc_core::serde::Serialize,
            RpcBalance: Send + Sync + 'static + _jsonrpc_core::serde::de::DeserializeOwned,
            BlockNumber: Send + Sync + 'static + _jsonrpc_core::serde::de::DeserializeOwned,
        {
            fn from(channel: RpcChannel) -> Self {
                Client::new(channel.into())
            }
        }
    }
    /// The generated server module.
    pub mod gen_server {
        use self::_jsonrpc_core::futures as _futures;
        use super::*;
        /// XMiningAsset RPC methods.
        pub trait XMiningAssetApi<BlockHash, AccountId, RpcBalance, BlockNumber>:
            Sized + Send + Sync + 'static
        {
            /// Get overall information about all mining assets.
            fn mining_assets(
                &self,
                at: Option<BlockHash>,
            ) -> Result<Vec<MiningAssetInfo<AccountId, RpcBalance, BlockNumber>>>;
            /// Get the asset mining dividends info given the asset miner AccountId.
            fn mining_dividend(
                &self,
                who: AccountId,
                at: Option<BlockHash>,
            ) -> Result<BTreeMap<AssetId, RpcBalance>>;
            /// Get the mining ledger details given the asset miner AccountId.
            fn miner_ledger(
                &self,
                who: AccountId,
                at: Option<BlockHash>,
            ) -> Result<BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>>;
            /// Create an `IoDelegate`, wiring rpc calls to the trait methods.
            fn to_delegate<M: _jsonrpc_core::Metadata>(self) -> _jsonrpc_core::IoDelegate<Self, M>
            where
                BlockHash: Send + Sync + 'static + _jsonrpc_core::serde::de::DeserializeOwned,
                AccountId: Send
                    + Sync
                    + 'static
                    + _jsonrpc_core::serde::Serialize
                    + _jsonrpc_core::serde::de::DeserializeOwned,
                RpcBalance: Send + Sync + 'static + _jsonrpc_core::serde::Serialize,
                BlockNumber: Send + Sync + 'static + _jsonrpc_core::serde::Serialize,
            {
                let mut del = _jsonrpc_core::IoDelegate::new(self.into());
                del.add_method("xminingasset_getMiningAssets", move |base, params| {
                    let method = &(Self::mining_assets
                        as fn(
                            &Self,
                            Option<BlockHash>,
                        )
                            -> Result<Vec<MiningAssetInfo<AccountId, RpcBalance, BlockNumber>>>);
                    let passed_args_num = match params {
                        _jsonrpc_core::Params::Array(ref v) => Ok(v.len()),
                        _jsonrpc_core::Params::None => Ok(0),
                        _ => Err(_jsonrpc_core::Error::invalid_params(
                            "`params` should be an array",
                        )),
                    };
                    let params =
                        passed_args_num.and_then(|passed_args_num| match passed_args_num {
                            _ if passed_args_num < 0usize => {
                                Err(_jsonrpc_core::Error::invalid_params({
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["`params` should have at least ", " argument(s)"],
                                        &match (&0usize,) {
                                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            )],
                                        },
                                    ));
                                    res
                                }))
                            }
                            0usize => params
                                .expect_no_params()
                                .map(|_| (None,))
                                .map_err(Into::into),
                            1usize => params
                                .parse::<(Option<BlockHash>,)>()
                                .map(|(a,)| (a,))
                                .map_err(Into::into),
                            _ => Err(_jsonrpc_core::Error::invalid_params_with_details(
                                {
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Expected from ", " to ", " parameters."],
                                        &match (&0usize, &1usize) {
                                            (arg0, arg1) => [
                                                ::core::fmt::ArgumentV1::new(
                                                    arg0,
                                                    ::core::fmt::Display::fmt,
                                                ),
                                                ::core::fmt::ArgumentV1::new(
                                                    arg1,
                                                    ::core::fmt::Display::fmt,
                                                ),
                                            ],
                                        },
                                    ));
                                    res
                                },
                                {
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Got: "],
                                        &match (&passed_args_num,) {
                                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            )],
                                        },
                                    ));
                                    res
                                },
                            )),
                        });
                    match params {
                        Ok((a,)) => {
                            use self::_futures::{Future, IntoFuture};
                            let fut = (method)(base, a)
                                .into_future()
                                .map(|value| {
                                    _jsonrpc_core::to_value(value)
                                        .expect("Expected always-serializable type; qed")
                                })
                                .map_err(Into::into as fn(_) -> _jsonrpc_core::Error);
                            _futures::future::Either::A(fut)
                        }
                        Err(e) => _futures::future::Either::B(_futures::failed(e)),
                    }
                });
                del.add_method("xminingasset_getDividendByAccount", move |base, params| {
                    let method = &(Self::mining_dividend
                        as fn(
                            &Self,
                            AccountId,
                            Option<BlockHash>,
                        ) -> Result<BTreeMap<AssetId, RpcBalance>>);
                    let passed_args_num = match params {
                        _jsonrpc_core::Params::Array(ref v) => Ok(v.len()),
                        _jsonrpc_core::Params::None => Ok(0),
                        _ => Err(_jsonrpc_core::Error::invalid_params(
                            "`params` should be an array",
                        )),
                    };
                    let params =
                        passed_args_num.and_then(|passed_args_num| match passed_args_num {
                            _ if passed_args_num < 1usize => {
                                Err(_jsonrpc_core::Error::invalid_params({
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["`params` should have at least ", " argument(s)"],
                                        &match (&1usize,) {
                                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            )],
                                        },
                                    ));
                                    res
                                }))
                            }
                            1usize => params
                                .parse::<(AccountId,)>()
                                .map(|(a,)| (a, None))
                                .map_err(Into::into),
                            2usize => params
                                .parse::<(AccountId, Option<BlockHash>)>()
                                .map(|(a, b)| (a, b))
                                .map_err(Into::into),
                            _ => Err(_jsonrpc_core::Error::invalid_params_with_details(
                                {
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Expected from ", " to ", " parameters."],
                                        &match (&1usize, &2usize) {
                                            (arg0, arg1) => [
                                                ::core::fmt::ArgumentV1::new(
                                                    arg0,
                                                    ::core::fmt::Display::fmt,
                                                ),
                                                ::core::fmt::ArgumentV1::new(
                                                    arg1,
                                                    ::core::fmt::Display::fmt,
                                                ),
                                            ],
                                        },
                                    ));
                                    res
                                },
                                {
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["Got: "],
                                        &match (&passed_args_num,) {
                                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            )],
                                        },
                                    ));
                                    res
                                },
                            )),
                        });
                    match params {
                        Ok((a, b)) => {
                            use self::_futures::{Future, IntoFuture};
                            let fut = (method)(base, a, b)
                                .into_future()
                                .map(|value| {
                                    _jsonrpc_core::to_value(value)
                                        .expect("Expected always-serializable type; qed")
                                })
                                .map_err(Into::into as fn(_) -> _jsonrpc_core::Error);
                            _futures::future::Either::A(fut)
                        }
                        Err(e) => _futures::future::Either::B(_futures::failed(e)),
                    }
                });
                del.add_method(
                    "xminingasset_getMinerLedgerByAccount",
                    move |base, params| {
                        let method = &(Self::miner_ledger
                            as fn(
                                &Self,
                                AccountId,
                                Option<BlockHash>,
                            )
                                -> Result<BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>>);
                        let passed_args_num = match params {
                            _jsonrpc_core::Params::Array(ref v) => Ok(v.len()),
                            _jsonrpc_core::Params::None => Ok(0),
                            _ => Err(_jsonrpc_core::Error::invalid_params(
                                "`params` should be an array",
                            )),
                        };
                        let params =
                            passed_args_num.and_then(|passed_args_num| match passed_args_num {
                                _ if passed_args_num < 1usize => {
                                    Err(_jsonrpc_core::Error::invalid_params({
                                        let res =
                                            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                                &["`params` should have at least ", " argument(s)"],
                                                &match (&1usize,) {
                                                    (arg0,) => [::core::fmt::ArgumentV1::new(
                                                        arg0,
                                                        ::core::fmt::Display::fmt,
                                                    )],
                                                },
                                            ));
                                        res
                                    }))
                                }
                                1usize => params
                                    .parse::<(AccountId,)>()
                                    .map(|(a,)| (a, None))
                                    .map_err(Into::into),
                                2usize => params
                                    .parse::<(AccountId, Option<BlockHash>)>()
                                    .map(|(a, b)| (a, b))
                                    .map_err(Into::into),
                                _ => Err(_jsonrpc_core::Error::invalid_params_with_details(
                                    {
                                        let res =
                                            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                                &["Expected from ", " to ", " parameters."],
                                                &match (&1usize, &2usize) {
                                                    (arg0, arg1) => [
                                                        ::core::fmt::ArgumentV1::new(
                                                            arg0,
                                                            ::core::fmt::Display::fmt,
                                                        ),
                                                        ::core::fmt::ArgumentV1::new(
                                                            arg1,
                                                            ::core::fmt::Display::fmt,
                                                        ),
                                                    ],
                                                },
                                            ));
                                        res
                                    },
                                    {
                                        let res =
                                            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                                &["Got: "],
                                                &match (&passed_args_num,) {
                                                    (arg0,) => [::core::fmt::ArgumentV1::new(
                                                        arg0,
                                                        ::core::fmt::Display::fmt,
                                                    )],
                                                },
                                            ));
                                        res
                                    },
                                )),
                            });
                        match params {
                            Ok((a, b)) => {
                                use self::_futures::{Future, IntoFuture};
                                let fut = (method)(base, a, b)
                                    .into_future()
                                    .map(|value| {
                                        _jsonrpc_core::to_value(value)
                                            .expect("Expected always-serializable type; qed")
                                    })
                                    .map_err(Into::into as fn(_) -> _jsonrpc_core::Error);
                                _futures::future::Either::A(fut)
                            }
                            Err(e) => _futures::future::Either::B(_futures::failed(e)),
                        }
                    },
                );
                del
            }
        }
    }
}
pub use self::rpc_impl_XMiningAssetApi::gen_client;
pub use self::rpc_impl_XMiningAssetApi::gen_server::XMiningAssetApi;
/// A struct that implements the [`XMiningAssetApi`].
pub struct XMiningAsset<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}
impl<C, B> XMiningAsset<C, B> {
    /// Create new `XMiningAsset` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}
impl<C, Block, AccountId, Balance, BlockNumber>
    XMiningAssetApi<<Block as BlockT>::Hash, AccountId, RpcBalance<Balance>, BlockNumber>
    for XMiningAsset<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: XMiningAssetRuntimeApi<Block, AccountId, Balance, BlockNumber>,
    AccountId: Codec,
    Balance: Codec,
    BlockNumber: Codec,
{
    fn mining_assets(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<MiningAssetInfo<AccountId, RpcBalance<Balance>, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api.mining_assets(&at).map_err(runtime_error_into_rpc_err)?)
    }
    fn mining_dividend(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, RpcBalance<Balance>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .mining_dividend(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
    }
    fn miner_ledger(
        &self,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BTreeMap<AssetId, RpcMinerLedger<BlockNumber>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        Ok(api
            .miner_ledger(&at, who)
            .map_err(runtime_error_into_rpc_err)?)
    }
}
/// Error type of this RPC api.
pub enum Error {
    /// The transaction was not decodable.
    DecodeError,
    /// The call to runtime failed.
    RuntimeError,
}
impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
            Error::DecodeError => 2,
        }
    }
}
const RUNTIME_ERROR: i64 = 1;
/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> RpcError {
    RpcError {
        code: ErrorCode::ServerError(RUNTIME_ERROR),
        message: "Runtime trapped".into(),
        data: Some(
            {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &[""],
                    &match (&err,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt)],
                    },
                ));
                res
            }
            .into(),
        ),
    }
}
