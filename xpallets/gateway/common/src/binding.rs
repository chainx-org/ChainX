// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{IterableStorageDoubleMap, StorageDoubleMap};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use chainx_primitives::{AssetId, ChainAddress, ReferralId};
use xpallet_assets::Chain;
use xpallet_support::{debug, error, info, str, traits::Validator, try_addr, warn};

use crate::traits::{AddrBinding, ChannelBinding};
use crate::{AddressBinding, BoundAddressOf, Module, Trait};

impl<T: Trait> ChannelBinding<T::AccountId> for Module<T> {
    fn update_binding(assert_id: &AssetId, who: &T::AccountId, channel_name: Option<ReferralId>) {
        let chain = match xpallet_assets_registrar::Module::<T>::chain_of(assert_id) {
            Ok(c) => c,
            Err(e) => {
                error!(
                    "[update_binding]|meed an unexpected asset_id:{:?}, error:{:?}",
                    assert_id, e
                );
                return;
            }
        };

        if let Some(name) = channel_name {
            if let Some(channel) = T::Validator::validator_for(&name) {
                match Self::channel_binding_of(who, chain) {
                    None => {
                        // set to storage
                        Self::set_binding(chain, who.clone(), channel);
                    }
                    Some(_channel) => {
                        debug!("[update_binding]|already has binding, do nothing|assert_id:{:}|chain:{:?}|who:{:?}|channel:{:?}", assert_id, chain, who, _channel);
                    }
                }
            } else {
                warn!(
                    "[update_binding]|channel not exist, do not set binding|name:{:?}",
                    str!(&name)
                );
            };
        };
    }

    fn get_binding_info(assert_id: &AssetId, who: &T::AccountId) -> Option<T::AccountId> {
        let chain = xpallet_assets_registrar::Module::<T>::chain_of(assert_id).ok()?;
        Self::channel_binding_of(who, chain)
    }
}

impl<T: Trait, Addr: Into<Vec<u8>>> AddrBinding<T::AccountId, Addr> for Module<T> {
    fn update_binding(chain: Chain, addr: Addr, who: T::AccountId) {
        let address = addr.into();
        if let Some(accountid) = AddressBinding::<T>::get(chain, &address) {
            if accountid != who {
                debug!(
                    "[apply_update_binding]|current binding need change|old:{:?}|new:{:?}",
                    accountid, who
                );
                // old accountid is not equal to new accountid, means should change this addr bind to new account
                // remove this addr for old accounid's CrossChainBindOf
                BoundAddressOf::<T>::mutate(accountid, chain, |addr_list| {
                    addr_list.retain(|addr| addr != &address);
                });
            }
        }
        // insert or override binding relationship
        BoundAddressOf::<T>::mutate(&who, chain, |addr_list| {
            if !addr_list.contains(&address) {
                addr_list.push(address.clone());
            }
        });

        info!(
            "[apply_update_binding]|update binding|chain:{:?}|addr:{:?}|who:{:?}",
            chain,
            try_addr!(address),
            who,
        );
        AddressBinding::<T>::insert(chain, address, who);
    }

    fn get_binding(chain: Chain, addr: Addr) -> Option<T::AccountId> {
        let addr_bytes: ChainAddress = addr.into();
        AddressBinding::<T>::get(chain, &addr_bytes)
    }
}

// export for runtime-api
impl<T: Trait> Module<T> {
    pub fn bound_addrs(who: &T::AccountId) -> BTreeMap<Chain, Vec<ChainAddress>> {
        BoundAddressOf::<T>::iter_prefix(&who).collect()
    }
}
