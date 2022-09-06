// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use frame_support::log::{debug, error, info, warn};
use scale_info::prelude::string::String;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use chainx_primitives::{AssetId, ChainAddress, ReferralId};
use xp_gateway_bitcoin::OpReturnAccount;
use xp_gateway_common::{transfer_aptos_uncheck, transfer_evm_uncheck, DstChain};
use xpallet_assets::Chain;
use xpallet_support::{traits::Validator, try_addr, try_str};

use crate::traits::{AddressBinding, ReferralBinding};
use crate::{
    AddressBindingOf, AddressBindingOfDstChain, BoundAddressOf, BoundAddressOfDstChain, Config,
    Pallet,
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
            OpReturnAccount::Evm(w) => Pallet::<T>::update_dst_chain_binding(
                chain,
                DstChain::ChainXEvm,
                address,
                w.as_bytes().to_vec(),
            ),
            OpReturnAccount::Wasm(w) => Pallet::<T>::update_wasm_binding(chain, address, w),
            OpReturnAccount::Aptos(w) => Pallet::<T>::update_dst_chain_binding(
                chain,
                DstChain::Aptos,
                address,
                w.as_bytes().to_vec(),
            ),
            OpReturnAccount::Named(prefix, w) => {
                // If there are multiple named types, consider replacing it to match
                if String::from_utf8_lossy(&prefix)
                    .to_ascii_lowercase()
                    .as_str()
                    == "sui"
                {
                    Pallet::<T>::update_dst_chain_binding(chain, DstChain::Sui, address, w)
                }
            }
        }
    }

    fn address(chain: Chain, address: Address) -> Option<OpReturnAccount<T::AccountId>> {
        let addr_bytes: ChainAddress = address.into();

        if AddressBindingOf::<T>::contains_key(chain, &addr_bytes) {
            if let Some(wasm_addr) = AddressBindingOf::<T>::get(chain, &addr_bytes) {
                return Some(OpReturnAccount::Wasm(wasm_addr));
            }
        }

        if AddressBindingOfDstChain::<T>::contains_key((chain, DstChain::ChainXEvm, &addr_bytes)) {
            if let Some(evm_raw_addr) =
                AddressBindingOfDstChain::<T>::get((chain, DstChain::ChainXEvm, &addr_bytes))
            {
                let evm_addr = transfer_evm_uncheck(&evm_raw_addr)?;
                return Some(OpReturnAccount::Evm(evm_addr));
            }
        }

        if AddressBindingOfDstChain::<T>::contains_key((chain, DstChain::Aptos, &addr_bytes)) {
            if let Some(aptos_raw_addr) =
                AddressBindingOfDstChain::<T>::get((chain, DstChain::Aptos, &addr_bytes))
            {
                let aptos_addr = transfer_aptos_uncheck(&aptos_raw_addr)?;
                return Some(OpReturnAccount::Aptos(aptos_addr));
            }
        }

        if AddressBindingOfDstChain::<T>::contains_key((chain, DstChain::Sui, &addr_bytes)) {
            if let Some(sui_addr) =
                AddressBindingOfDstChain::<T>::get((chain, DstChain::Sui, &addr_bytes))
            {
                return Some(OpReturnAccount::Named("sui".as_bytes().to_vec(), sui_addr));
            }
        }

        None
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

    fn update_dst_chain_binding<Address>(
        chain: Chain,
        dst_chain: DstChain,
        address: Address,
        who: ChainAddress,
    ) where
        Address: Into<Vec<u8>>,
    {
        let address = address.into();
        if let Some(accountid) = AddressBindingOfDstChain::<T>::get((chain, dst_chain, &address)) {
            if accountid != who {
                debug!(
                    target: "runtime::gateway::common",
                    "[update_address_binding] Current address binding need to changed (old:{:?} => new:{:?})",
                    accountid, who
                );
                // old accountid is not equal to new accountid, means should change this addr bind to new account
                // remove this addr for old accounid's CrossChainBindOf
                BoundAddressOfDstChain::<T>::mutate((accountid, chain, dst_chain), |addr_list| {
                    addr_list.retain(|addr| addr != &address);
                });
            }
        }
        // insert or override binding relationship
        BoundAddressOfDstChain::<T>::mutate((&who, chain, dst_chain), |addr_list| {
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
        AddressBindingOfDstChain::<T>::insert((chain, dst_chain, address), who);
    }
}
