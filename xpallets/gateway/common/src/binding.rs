// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::log::{debug, error, info, warn};
use sp_core::{H160, H256};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use chainx_primitives::{AssetId, ChainAddress, ReferralId};
use xp_gateway_bitcoin::OpReturnAccount;
use xpallet_assets::Chain;
use xpallet_support::{traits::Validator, try_addr, try_str};

use crate::traits::{AddressBinding, ReferralBinding};
use crate::{
    AddressBindingOf, AddressBindingOfAptos, AddressBindingOfEvm, BoundAddressOf,
    BoundAddressOfAptos, BoundAddressOfEvm, Config, Pallet,
};

impl<T: Config> ReferralBinding<T::AccountId> for Pallet<T> {
    fn update_binding(asset_id: &AssetId, who: &T::AccountId, referral_name: Option<ReferralId>) {
        let chain = match xpallet_assets_registrar::Pallet::<T>::chain_of(asset_id) {
            Ok(chain) => chain,
            Err(err) => {
                error!(
                    target: "runtime::gateway::common",
                    "[update_referral_binding] Unexpected asset_id:{:?}, error:{:?}",
                    asset_id, err
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
                            target: "runtime::gateway::common",
                            "[update_referral_binding] Already has referral binding:[assert id:{}, chain:{:?}, who:{:?}, referral:{:?}]",
                            asset_id, chain, who, channel
                        );
                    }
                }
            } else {
                warn!(
                    target: "runtime::gateway::common",
                    "[update_referral_binding] {:?} has no referral, cannot update binding",
                    try_str(name)
                );
            };
        };
    }

    fn referral(asset_id: &AssetId, who: &T::AccountId) -> Option<T::AccountId> {
        let chain = xpallet_assets_registrar::Pallet::<T>::chain_of(asset_id).ok()?;
        Self::referral_binding_of(who, chain)
    }
}

impl<T: Config, Address: Into<Vec<u8>>> AddressBinding<T::AccountId, Address> for Pallet<T> {
    fn update_binding(chain: Chain, address: Address, who: OpReturnAccount<T::AccountId>) {
        match who {
            OpReturnAccount::Evm(w) => Pallet::<T>::update_evm_binding(chain, address, w),
            OpReturnAccount::Wasm(w) => Pallet::<T>::update_wasm_binding(chain, address, w),
            OpReturnAccount::Aptos(w) => Pallet::<T>::update_aptos_binding(chain, address, w),
            OpReturnAccount::Named(p, w) => Pallet::<T>::update_named_binding(),
        }
    }

    fn address(chain: Chain, address: Address) -> Option<OpReturnAccount<T::AccountId>> {
        let addr_bytes: ChainAddress = address.into();
        match addr_bytes.len() {
            20 => Some(OpReturnAccount::Evm(AddressBindingOfEvm::<T>::get(
                chain,
                &addr_bytes,
            )?)),
            32 => Some(OpReturnAccount::Aptos(AddressBindingOfAptos::<T>::get(
                chain,
                &addr_bytes,
            )?)),
            _ => Some(OpReturnAccount::Wasm(AddressBindingOf::<T>::get(
                chain,
                &addr_bytes,
            )?)),
        }
    }
}

// export for runtime-api
impl<T: Config> Pallet<T> {
    // todo! Add find of evm address
    pub fn bound_addrs(who: &T::AccountId) -> BTreeMap<Chain, Vec<ChainAddress>> {
        BoundAddressOf::<T>::iter_prefix(&who).collect()
    }

    fn update_wasm_binding<Address>(chain: Chain, address: Address, who: T::AccountId)
    where
        Address: Into<Vec<u8>>,
    {
        let address = address.into();
        if let Some(accountid) = AddressBindingOf::<T>::get(chain, &address) {
            if accountid != who {
                debug!(
                    target: "runtime::gateway::common",
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
            target: "runtime::gateway::common",
            "[update_address_binding] Update address binding:[chain:{:?}, addr:{:?}, who:{:?}]",
            chain,
            try_addr(&address),
            who,
        );
        AddressBindingOf::<T>::insert(chain, address, who);
    }

    fn update_evm_binding<Address>(chain: Chain, address: Address, who: H160)
    where
        Address: Into<Vec<u8>>,
    {
        let address = address.into();
        if let Some(accountid) = AddressBindingOfEvm::<T>::get(chain, &address) {
            if accountid != who {
                debug!(
                    target: "runtime::gateway::common",
                    "[update_address_binding] Current address binding need to changed (old:{:?} => new:{:?})",
                    accountid, who
                );
                // old accountid is not equal to new accountid, means should change this addr bind to new account
                // remove this addr for old accounid's CrossChainBindOf
                BoundAddressOfEvm::<T>::mutate(accountid, chain, |addr_list| {
                    addr_list.retain(|addr| addr != &address);
                });
            }
        }
        // insert or override binding relationship
        BoundAddressOfEvm::<T>::mutate(&who, chain, |addr_list| {
            if !addr_list.contains(&address) {
                addr_list.push(address.clone());
            }
        });

        info!(
            target: "runtime::gateway::common",
            "[update_address_binding] Update address binding:[chain:{:?}, addr:{:?}, who:{:?}]",
            chain,
            try_addr(&address),
            who,
        );
        AddressBindingOfEvm::<T>::insert(chain, address, who);
    }

    fn update_aptos_binding<Address>(chain: Chain, address: Address, who: H256)
    where
        Address: Into<Vec<u8>>,
    {
        let address = address.into();
        if let Some(accountid) = AddressBindingOfAptos::<T>::get(chain, &address) {
            if accountid != who {
                debug!(
                    target: "runtime::gateway::common",
                    "[update_address_binding] Current address binding need to changed (old:{:?} => new:{:?})",
                    accountid, who
                );
                // old accountid is not equal to new accountid, means should change this addr bind to new account
                // remove this addr for old accounid's CrossChainBindOf
                BoundAddressOfAptos::<T>::mutate(accountid, chain, |addr_list| {
                    addr_list.retain(|addr| addr != &address);
                });
            }
        }
        // insert or override binding relationship
        BoundAddressOfAptos::<T>::mutate(&who, chain, |addr_list| {
            if !addr_list.contains(&address) {
                addr_list.push(address.clone());
            }
        });

        info!(
            target: "runtime::gateway::common",
            "[update_address_binding] Update address binding:[chain:{:?}, addr:{:?}, who:{:?}]",
            chain,
            try_addr(&address),
            who,
        );
        AddressBindingOfAptos::<T>::insert(chain, address, who);
    }

    fn update_named_binding() {
        todo!()
    }
}
