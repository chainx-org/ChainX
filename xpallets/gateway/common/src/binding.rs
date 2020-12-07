// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::{IterableStorageDoubleMap, StorageDoubleMap};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use chainx_primitives::{AssetId, ChainAddress, ReferralId};
use xp_logging::{debug, error, info, warn};
use xpallet_assets::Chain;
use xpallet_support::{traits::Validator, try_addr, try_str};

use crate::traits::{AddressBinding, ReferralBinding};
use crate::{AddressBindingOf, BoundAddressOf, Module, Trait};

impl<T: Trait> ReferralBinding<T::AccountId> for Module<T> {
    fn update_binding(assert_id: &AssetId, who: &T::AccountId, referral_name: Option<ReferralId>) {
        let chain = match xpallet_assets_registrar::Module::<T>::chain_of(assert_id) {
            Ok(chain) => chain,
            Err(err) => {
                error!(
                    "[update_referral_binding] Unexpected asset_id:{:?}, error:{:?}",
                    assert_id, err
                );
                return;
            }
        };

        if let Some(name) = referral_name {
            if let Some(referral) = T::Validator::validator_for(&name) {
                match Self::referral_binding_of(who, chain) {
                    None => {
                        // set to storage
                        Self::set_referral_binding(chain, who.clone(), referral);
                    }
                    Some(channel) => {
                        debug!(
                            "[update_referral_binding] Already has referral binding:[assert id:{}, chain:{:?}, who:{:?}, referral:{:?}]",
                            assert_id, chain, who, channel
                        );
                    }
                }
            } else {
                warn!(
                    "[update_referral_binding] {:?} has no referral, cannot update binding",
                    try_str(name)
                );
            };
        };
    }

    fn referral(assert_id: &AssetId, who: &T::AccountId) -> Option<T::AccountId> {
        let chain = xpallet_assets_registrar::Module::<T>::chain_of(assert_id).ok()?;
        Self::referral_binding_of(who, chain)
    }
}

impl<T: Trait, Address: Into<Vec<u8>>> AddressBinding<T::AccountId, Address> for Module<T> {
    fn update_binding(chain: Chain, address: Address, who: T::AccountId) {
        let address = address.into();
        if let Some(accountid) = AddressBindingOf::<T>::get(chain, &address) {
            if accountid != who {
                debug!(
                    "[update_address_binding] Current address binding need to changed (old:{:?} => new:{:?})",
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
            "[update_address_binding] Update address binding:[chain:{:?}, addr:{:?}, who:{:?}]",
            chain,
            try_addr(&address),
            who,
        );
        AddressBindingOf::<T>::insert(chain, address, who);
    }

    fn address(chain: Chain, address: Address) -> Option<T::AccountId> {
        let addr_bytes: ChainAddress = address.into();
        AddressBindingOf::<T>::get(chain, &addr_bytes)
    }
}

// export for runtime-api
impl<T: Trait> Module<T> {
    pub fn bound_addrs(who: &T::AccountId) -> BTreeMap<Chain, Vec<ChainAddress>> {
        BoundAddressOf::<T>::iter_prefix(&who).collect()
    }
}
