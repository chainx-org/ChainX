extern crate substrate_runtime_balances as balances;
extern crate substrate_runtime_primitives;
extern crate substrate_keyring;
extern crate substrate_network;
extern crate substrate_codec;

extern crate chainx_primitives;
extern crate chainx_runtime;
extern crate chainx_pool;

use substrate_network::TransactionPool as Pool;
use substrate_runtime_primitives::MaybeUnsigned;
use substrate_keyring::Keyring;
use substrate_codec::Encode;

use chainx_runtime::{Extrinsic, UncheckedExtrinsic, BareExtrinsic, Concrete, Call};
use chainx_pool::TransactionPool;
use chainx_primitives::AccountId;

use std::sync::Arc;

fn alice() -> AccountId {
    AccountId::from(Keyring::Alice.to_raw_public())
}

fn xt() -> UncheckedExtrinsic {
    let extrinsic = BareExtrinsic {
        signed: alice(),
        index: 0,
        function: Call::Balances(balances::Call::transfer::<Concrete>(alice().into(), 69)),
    };
    let signature = MaybeUnsigned(Keyring::from_raw_public(extrinsic.signed.0.clone()).unwrap()
        .sign(&extrinsic.encode()).into());
    let extrinsic = Extrinsic {
        signed: extrinsic.signed.into(),
        index: extrinsic.index,
        function: extrinsic.function,
    };
    UncheckedExtrinsic::new(extrinsic, signature)
}


pub fn push_one_transaction(extrinsic_pool: Arc<TransactionPool>){
    let _txhash = extrinsic_pool.clone().import(&xt().encode()).unwrap();
}
