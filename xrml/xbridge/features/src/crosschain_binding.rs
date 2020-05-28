use parity_codec::Codec;
use rstd::prelude::Vec;

use support::StorageMap;

use primitives::traits::MaybeDebug;
use xassets::Chain;
use xr_primitives::Name;
use xsupport::{debug, trace};

use xbridge_common::traits::{AsRefAndMutOption, CrossChainBinding};

pub use btc_keys::Address as BitcoinAddress;
pub use xsdot::types::EthereumAddress;

use super::{Module, RawEvent, Trait};
// bitocin
use super::{BitcoinCrossChainBinding, BitcoinCrossChainOf};
// ethereum
use super::{EthereumCrossChainBinding, EthereumCrossChainOf};

impl<T: Trait> Module<T> {
    /// Actually update the binding address of original transactor.
    fn apply_update_binding<
        CrossChainOf,
        CrossChainBinding,
        ChainAddress: Clone + Codec + PartialEq + MaybeDebug,
    >(
        who: &T::AccountId,
        address: ChainAddress,
        channel_name: Option<Name>,
    ) -> (
        T::AccountId,
        Option<T::AccountId>,
        ChainAddress,
        Option<T::AccountId>,
    )
    where
        CrossChainOf: StorageMap<ChainAddress, (T::AccountId, Option<T::AccountId>)>,
        <CrossChainOf as StorageMap<ChainAddress, (T::AccountId, Option<T::AccountId>)>>::Query:
            AsRefAndMutOption<(T::AccountId, Option<T::AccountId>)>,
        CrossChainBinding: StorageMap<T::AccountId, Vec<ChainAddress>>,
        <CrossChainBinding as StorageMap<T::AccountId, Vec<ChainAddress>>>::Query:
            AsRef<Vec<ChainAddress>> + AsMut<Vec<ChainAddress>>,
    {
        if let Some((accountid, _)) = CrossChainOf::get(&address).as_ref() {
            if accountid != who {
                debug!(
                    "[apply_update_binding]|current binding need change|old:{:?}|new:{:?}",
                    accountid, who
                );
                // old accountid is not equal to new accountid, means should change this addr bind to new account
                // remove this addr for old accounid's CrossChainBindOf
                CrossChainBinding::mutate(accountid, |addr_list| {
                    addr_list.as_mut().retain(|addr| addr != &address);
                });
            }
        }
        // insert or override binding relationship
        CrossChainBinding::mutate(who, |addr_list| {
            let list = addr_list.as_mut();
            if !list.contains(&address) {
                list.push(address.clone());
            }
        });

        let channel_accountid = channel_name.and_then(xaccounts::Module::<T>::intention_of);
        debug!(
            "[apply_update_binding]|update binding|addr:{:?}|who:{:?}|channel:{:?}",
            address, who, channel_accountid
        );
        CrossChainOf::insert(&address, (who.clone(), channel_accountid.clone()));
        (who.clone(), None, address, channel_accountid)
    }

    fn get_first_binding_channel_impl<
        CrossChainOf,
        CrossChainBinding,
        ChainAddress: Clone + Codec + PartialEq,
    >(
        who: &T::AccountId,
    ) -> Option<T::AccountId>
    where
        CrossChainOf: StorageMap<ChainAddress, (T::AccountId, Option<T::AccountId>)>,
        <CrossChainOf as StorageMap<ChainAddress, (T::AccountId, Option<T::AccountId>)>>::Query:
            AsRefAndMutOption<(T::AccountId, Option<T::AccountId>)>,
        CrossChainBinding: StorageMap<T::AccountId, Vec<ChainAddress>>,
        <CrossChainBinding as StorageMap<T::AccountId, Vec<ChainAddress>>>::Query:
            AsRef<Vec<ChainAddress>> + AsMut<Vec<ChainAddress>>,
    {
        let bind = CrossChainBinding::get(who);
        // get first binding
        if let Some(first_bind) = bind.as_ref().get(0) {
            if let Some((_, channel_accountid)) = CrossChainOf::get(first_bind).as_ref() {
                // if the channel_accountid is `Option<T::AccountId>`
                return channel_accountid.clone();
            }
        }
        None
    }

    pub fn get_first_binding_channel(who: &T::AccountId, chain: Chain) -> Option<T::AccountId> {
        let channel_info = match chain {
            Chain::Bitcoin => Self::get_first_binding_channel_impl::<
                BitcoinCrossChainOf<T>,
                BitcoinCrossChainBinding<T>,
                BitcoinAddress,
            >(who),
            Chain::Ethereum => Self::get_first_binding_channel_impl::<
                EthereumCrossChainOf<T>,
                EthereumCrossChainBinding<T>,
                EthereumAddress,
            >(who),
            _ => None,
        };
        trace!(
            "[first_channel_binding]|who:{:?}|chain:{:?}|channel:{:?}",
            who,
            chain,
            channel_info
        );
        channel_info
    }
}

impl<T: Trait> CrossChainBinding<T::AccountId, BitcoinAddress> for Module<T> {
    fn update_binding(who: &T::AccountId, addr: BitcoinAddress, channel_name: Option<Name>) {
        let (new_accountid, old_accountid, addr, channel) = Self::apply_update_binding::<
            BitcoinCrossChainOf<T>,
            BitcoinCrossChainBinding<T>,
            BitcoinAddress,
        >(who, addr, channel_name);
        Self::deposit_event(RawEvent::BitcoinBinding(
            new_accountid,
            old_accountid,
            addr,
            channel,
        ));
    }

    fn get_binding_info(
        input_addr: &BitcoinAddress,
    ) -> Option<(T::AccountId, Option<T::AccountId>)> {
        BitcoinCrossChainOf::<T>::get(input_addr)
    }
}

impl<T: Trait> CrossChainBinding<T::AccountId, EthereumAddress> for Module<T> {
    fn update_binding(who: &T::AccountId, addr: EthereumAddress, channel_name: Option<Name>) {
        let (new_accountid, old_accountid, addr, channel) = Self::apply_update_binding::<
            EthereumCrossChainOf<T>,
            EthereumCrossChainBinding<T>,
            EthereumAddress,
        >(who, addr, channel_name);
        Self::deposit_event(RawEvent::EthereumBinding(
            new_accountid,
            old_accountid,
            addr,
            channel,
        ));
    }

    fn get_binding_info(
        input_addr: &EthereumAddress,
    ) -> Option<(T::AccountId, Option<T::AccountId>)> {
        EthereumCrossChainOf::<T>::get(input_addr)
    }
}
