pub mod types;

use parity_codec::Decode;
// substrate
use primitives::traits::MaybeDebug;
use sp_std::{prelude::*, result};
use support::{decl_event, decl_module, decl_storage, dispatch::Result, StorageMap, StorageValue};
use system::ensure_signed;

// light-bitcoin
use btc_chain::Transaction;
use btc_keys::{Address as BTCAddress, Network};
use btc_primitives::H256;
use btc_script::Script;

use xrml_assets::{Chain, ChainT};
use xbridge_common::traits::{CrossChainBindingV2, Extractable};
use xr_primitives::Name;
#[cfg(feature = "std")]
use xsupport::try_hex_or_str;
use xsupport::{debug, error, info, warn};

use crate::tx::handler::TxHandler;
use crate::tx::utils::{
    addr2vecu8, get_networkid, parse_opreturn, parse_output_addr_with_networkid,
};
use crate::types::TxType;
use crate::{Module as XBitcoin, Trait as XBitcoinTrait};

use self::types::LockupRelayTx;

pub trait Trait: XBitcoinTrait + xbridge_common::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId {
        /// accountid, value, txid, txoutput_index, output_btc_addr
        Lock(AccountId, u64, H256, u32, Vec<u8>),
        /// txid, input_index, input_outpoint_hash, input_outpoint_index
        Unlock(H256, u32, H256, u32),
        /// use root to unlock UTXO
        UnlockedFromRoot(H256, u32),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// if use `LockupRelayTx` struct would export in metadata, cause complex in front-end
        pub fn push_transaction(origin, tx: Vec<u8>) -> Result {
            let from = ensure_signed(origin)?;
            let relay_tx: LockupRelayTx = Decode::decode(&mut tx.as_slice()).ok_or("Parse LockupRelayTx err")?;

            debug!("[push_transaction|lockup]|from:{:?}|relay_tx:{:?}", from, relay_tx);

            XBitcoin::<T>::apply_push_transaction(relay_tx)?;

            // 50 is trick number for call difficulty power, if change in `runtime/src/fee.rs`,
            // should modify this number.
            xbridge_common::Module::<T>::reward_relayer(&Self::TOKEN.to_vec(), &from, 50, tx.len() as u64);
            Ok(())
        }

        pub fn release_lock(utxos: Vec<(H256, u32)>) {
            for utxo in utxos {
                if destroy_utxo::<T>(utxo.0, utxo.1) {
                    Self::deposit_event(RawEvent::UnlockedFromRoot(utxo.0, utxo.1));
                }
            }
        }

        pub fn set_locked_coin_limit(limit: (u64, u64)) {
            LockedCoinLimit::<T>::put(&limit);
            info!("[set_locked_coin_limit]|set new lockup bitoin limit to:{:?}", limit);
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XBridgeOfBitcoinLockup {
        /// locked up btc is UTXO, using (txid, out_index) to represent it
        /// use UTXO => (accountid, balance)
        pub LockedUpBTC get(locked_up_btc): map (H256, u32) => Option<(T::AccountId, u64, BTCAddress)>;
        /// sum value for single Bitcoin addr
        pub AddressLockedCoin get(address_locked_coin): map BTCAddress => u64;

        /// single addr and ont output limit coin value, default limit is 0.01 BTC ~ 10 BTC
        pub LockedCoinLimit get(locked_coin_limit): (u64, u64) = (1*1000000, 10*100000000);
    }
}

impl<T: Trait> ChainT for Module<T> {
    const TOKEN: &'static [u8] = b"L-BTC";

    fn chain() -> Chain {
        Chain::Bitcoin
    }

    fn check_addr(_: &[u8], _: &[u8]) -> Result {
        unreachable!("L-BTC should not call `check_addr`");
    }
}

pub fn detect_lockup_type<T: Trait>(tx: &Transaction) -> TxType {
    let network = get_networkid::<T>();
    let addr_type = xsystem::Module::<T>::address_type();
    let value_limit = Module::<T>::locked_coin_limit();

    if detect_lock_tx::<T::AccountId, _>(
        tx,
        network,
        addr_type,
        T::AccountExtractor::account_info,
        value_limit,
    ) {
        return TxType::Lock;
    }

    if detect_unlock_tx::<T>(tx) {
        return TxType::Unlock;
    }

    warn!(
        "[detect_lockup_type]|it's an irrelevance tx|tx_hash:{:?}",
        tx.hash()
    );
    TxType::Irrelevance
}

/// detect lock bitcoin tx
/// rule: 1. input should not more than 10 ( input_len <= 10)
/// 2. output should be 2 or 3(2<= output_len <=3), must have only one opreturn in them
/// 3. opreturn addr[0..4] should match one of output addr, if same, choose first one
/// 4 lock output value should be 0.1 btc <= value <= 10 btc
/// 4.1 same addr should not more than 10 btc
pub fn detect_lock_tx<AccountId, F>(
    tx: &Transaction,
    network: Network,
    chainx_addr_type: u8,
    parse_account_info: F,
    limit: (u64, u64),
) -> bool
where
    AccountId: Default + MaybeDebug,
    F: Fn(&[u8], u8) -> Option<(AccountId, Option<Name>)>,
{
    let out_len = tx.outputs.len();
    if out_len < 2 || out_len > 3 {
        debug!(
            "[detect_lock_tx]|tx output count not 2 or 3|len:{:}",
            out_len
        );
        return false;
    }

    let in_len = tx.inputs.len();
    if in_len > 10 {
        debug!(
            "[detect_lock_tx]|tx input count should not more than 10|len:{:}",
            in_len
        );
        return false;
    }

    if let Some(_info) =
        parse_lock_info::<AccountId, F>(tx, network, chainx_addr_type, parse_account_info, limit)
    {
        debug!(
            "[detect_lock_tx]|it's lock tx|who:{:?}|value:{:}|addr:{:?}|tx_hash:{:}",
            (_info.2).0,
            tx.outputs[_info.1].value,
            _info.0,
            tx.hash()
        );
        return true;
    }
    debug!("[detect_lock_tx]|this tx is not lock type or can't parse any valid addr in all output|tx_hash:{:}", tx.hash());
    false
}

/// any utxo in storage would mark this tx is an unlock tx
pub fn detect_unlock_tx<T: Trait>(tx: &Transaction) -> bool {
    for input in tx.inputs.iter() {
        let key = (input.previous_output.hash, input.previous_output.index);
        if Module::<T>::locked_up_btc(key).is_some() {
            debug!(
                "[detect_unlock_tx]|it's an unlock tx|tx hash:{:}",
                tx.hash()
            );
            return true;
        }
    }
    debug!("[detect_unlock_tx]|this tx is not unlock type or there is some not confirmed UTXO|tx_hash:{:}", tx.hash());
    false
}

pub fn handle_lockup_tx<T: Trait>(tx_handle: &TxHandler) -> Result {
    match tx_handle.tx_info.tx_type {
        TxType::Lock => handle_lock_tx::<T>(&tx_handle.tx_info.raw_tx, &tx_handle.tx_hash)?,
        TxType::Unlock => handle_unlock_tx::<T>(&tx_handle.tx_info.raw_tx, &tx_handle.tx_hash),
        _ => panic!("[handle_lockup_tx]|should not handle type expect `Lock`|`Unlock`"),
    }
    Ok(())
}

pub(crate) fn handle_lock_tx<T: Trait>(
    tx: &Transaction,
    tx_hash: &H256,
) -> result::Result<(), &'static str> {
    let network = get_networkid::<T>();
    let addr_type = xsystem::Module::<T>::address_type();
    let value_limit = Module::<T>::locked_coin_limit();

    let (addr, out_index, account_info) = parse_lock_info::<T::AccountId, _>(
        tx,
        network,
        addr_type,
        T::AccountExtractor::account_info,
        value_limit,
    )
    .ok_or_else(|| {
        error!(
            "[handle_lock_tx]|parse lock info should not fail at here|hash:{:?}|tx:{:?}",
            tx_hash, tx
        );
        "parse lock info should not fail at here"
    })?;
    let output_value = tx.outputs[out_index].value;

    // set storage and issue token

    // try to unlock tx before new issue, if any error in it, just print error log
    // it's unlock and lock tx
    handle_unlock_tx::<T>(tx, tx_hash);

    // new value should not more than single addr limit
    let current_value = Module::<T>::address_locked_coin(addr);
    let addr_value = current_value + output_value;
    if addr_value > value_limit.1 {
        error!("[handle_lock_tx]|lock value more than single addr limit|cur_value:{:}|try lock:{:}|addr:{:?}", current_value, output_value, addr);
        return Err("lock value more than single addr limit");
    }

    let (accountid, channel) = account_info;
    let key = (*tx_hash, out_index as u32);
    LockedUpBTC::<T>::insert(&key, (accountid.clone(), output_value, addr));
    AddressLockedCoin::<T>::insert(addr, addr_value);
    // issue lockup token
    update_binding::<T>(&accountid, channel);
    issue_token::<T>(&accountid, output_value);

    Module::<T>::deposit_event(RawEvent::Lock(
        accountid.clone(),
        output_value,
        *tx_hash,
        out_index as u32,
        addr2vecu8(&addr),
    ));

    Ok(())
}

pub fn handle_unlock_tx<T: Trait>(tx: &Transaction, tx_hash: &H256) {
    debug!("[handle_unlock_tx]|do unlock tx|tx_hash:{:}", tx_hash);
    // delete utxo storage and destroy token
    for (index, input) in tx.inputs.iter().enumerate() {
        if destroy_utxo::<T>(input.previous_output.hash, input.previous_output.index) {
            Module::<T>::deposit_event(RawEvent::Unlock(
                *tx_hash,
                index as u32,
                input.previous_output.hash,
                input.previous_output.index,
            ));
        }
    }
}

fn destroy_utxo<T: Trait>(hash: H256, index: u32) -> bool {
    let key = (hash, index);
    if let Some((accountid, value, addr)) = LockedUpBTC::<T>::take(&key) {
        let addr_value = AddressLockedCoin::<T>::take(&addr);
        if let Some(v) = addr_value.checked_sub(value) {
            if v > 0 {
                AddressLockedCoin::<T>::insert(&addr, v);
            }
        }
        debug!(
            "[destroy_utxo]|unlock utxo|tx_hash:{:}|index:{:}",
            hash, index
        );
        destroy_token::<T>(&accountid, value);
        true
    } else {
        false
    }
}

fn parse_lock_info<AccountId, F>(
    tx: &Transaction,
    network: Network,
    chainx_addr_type: u8,
    parse_account_info: F,
    limit: (u64, u64),
) -> Option<(BTCAddress, usize, (AccountId, Option<Name>))>
where
    AccountId: Default + MaybeDebug,
    F: Fn(&[u8], u8) -> Option<(AccountId, Option<Name>)>,
{
    let opreturns = tx
        .outputs()
        .iter()
        .filter(|output| {
            let script: Script = output.script_pubkey.to_vec().into();
            script.is_null_data_script()
        })
        .collect::<Vec<_>>();

    if opreturns.len() != 1 {
        warn!(
            "[parse_lock_info]|output len more than once, may be an invalid tx|outpout len:{:}",
            opreturns.len()
        );
        return None;
    }

    let opreturn_script = opreturns[0].script_pubkey.to_vec().into();
    let (account_info, opreturn_addr) = match parse_opreturn(&opreturn_script)
        .and_then(|v| parse_opreturn_info(&v, chainx_addr_type, parse_account_info))
    {
        Some(r) => r,
        None => {
            warn!("[parse_lock_info]|parse opreturn failed, not an valid opreturn");
            return None;
        }
    };

    for (index, output) in tx.outputs().iter().enumerate() {
        // out script
        let script: Script = output.script_pubkey.to_vec().into();
        if !script.is_null_data_script() {
            // only allow p2pk p2pkh p2sh
            if let Some(addr) = parse_output_addr_with_networkid(&script, network) {
                // compare addr
                let addr_v = addr2vecu8(&addr);
                if &addr_v[..4] == &opreturn_addr[..] {
                    debug!("[parse_lock_info]|it's a lock tx");
                    // value should be 0.1 <= value <= 10
                    let value = output.value;
                    if limit.0 <= value && value <= limit.1 {
                        return Some((addr, index, account_info));
                    } else {
                        warn!("[parse_lock_info]|it's a lock tx but output value not match limit|value:{:}|limit:{:?}", value, limit);
                    }
                }
            }
        }
    }
    None
}

/// parse data in opreturn
/// the data should like:
/// `ChainX:chainx_addr[@channel]:btc_addr[0..4]`
///   v[0]          v[1]            v[2]
fn parse_opreturn_info<AccountId, F>(
    data: &[u8],
    chainx_addr_type: u8,
    parse_account_info: F,
) -> Option<((AccountId, Option<Name>), Vec<u8>)>
where
    AccountId: Default + MaybeDebug,
    F: Fn(&[u8], u8) -> Option<(AccountId, Option<Name>)>,
{
    let v = split(data);
    if v.len() != 3 {
        warn!(
            "[parse_opreturn_info]|not ChainX lockup opreturn|data:{:}",
            try_hex_or_str(data)
        );
        return None;
    }
    if v[0] != b"ChainX" {
        warn!(
            "[parse_opreturn_info]|protocol-prefix not match to lockup protocol|prefix:{:}",
            try_hex_or_str(&v[0])
        );
        return None;
    }
    // addr just choose the first 4 bytes
    if v[2].len() != 4 {
        warn!(
            "[parse_opreturn_info]|btc addr slice length is not 4 bytes|addr_slice:{:}",
            try_hex_or_str(&v[2])
        );
        return None;
    }

    parse_account_info(&v[1], chainx_addr_type).map(|info| (info, v[2].clone()))
}
#[inline]
fn split(data: &[u8]) -> Vec<Vec<u8>> {
    data.split(|x| *x == b':').map(|d| d.to_vec()).collect()
}

/// bind account
fn update_binding<T: Trait>(who: &T::AccountId, channel_name: Option<Name>) {
    let token: xrml_assets::Token = <Module<T> as xrml_assets::ChainT>::TOKEN.to_vec();
    xbridge_common::Module::<T>::update_binding(&token, who, channel_name);
}

fn issue_token<T: Trait>(who: &T::AccountId, balance: u64) {
    // notice this `Module` is LockupModule
    let token: xrml_assets::Token = <Module<T> as xrml_assets::ChainT>::TOKEN.to_vec();
    let _ = xrml_assets::Module::<T>::issue(&token, who, balance.into()).map_err(|e| {
        error!("{:}", e);
        e
    });
}

fn destroy_token<T: Trait>(who: &T::AccountId, balance: u64) {
    // notice this `Module` is LockupModule
    let token: xrml_assets::Token = <Module<T> as xrml_assets::ChainT>::TOKEN.to_vec();
    let _ = xrml_assets::Module::<T>::destroy_free(&token, who, balance.into()).map_err(|e| {
        error!("{:}", e);
        e
    });
}
