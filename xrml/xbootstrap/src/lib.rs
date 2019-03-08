// Copyright 2019 Chainpool.

//! this module is for bootstrap only.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use sr_io as runtime_io;
#[cfg(feature = "std")]
use sr_primitives as runtime_primitives;

#[cfg(feature = "std")]
use chain as btc_chain;
use runtime_support::{decl_module, decl_storage};
use srml_support as runtime_support;
//#[cfg(feature = "std")]
//use srml_system as system;
#[cfg(feature = "std")]
use substrate_finality_grandpa_primitives as fg_primitives;
#[cfg(feature = "std")]
use xrml_bridge_bitcoin as xbitcoin;
#[cfg(feature = "std")]
use xrml_mining_staking as xstaking;
use xrml_mining_tokens as xtokens;
#[cfg(feature = "std")]
use xrml_xaccounts as xaccounts;
#[cfg(feature = "std")]
use xrml_xassets_assets as xassets;
#[cfg(feature = "std")]
use xrml_xdex_spot as xspot;
use xrml_xmultisig as xmultisig;

pub trait Trait: xtokens::Trait + xmultisig::Trait {}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as XBootstrap {
    }

    add_extra_genesis {
        // xassets
        config(asset_list): Vec<(xassets::Asset, bool, bool, Vec<(T::AccountId, u64)>)>;
        config(pcx): (xassets::Token, xassets::Precision, xassets::Desc);

        // xstaking
        config(intentions): Vec<(T::AccountId, T::Balance, xaccounts::Name, xaccounts::URL)>;
        config(trustee_intentions): Vec<(T::AccountId, Vec<u8>, Vec<u8>)>;

        // xtokens
        config(endowed_users): Vec<(xassets::Token, Vec<(T::AccountId, T::Balance)>)>;

        // xspot
        config(pair_list): Vec<(xassets::Token, xassets::Token, u32, u32, T::Price, bool)>;

        // grandpa
        config(authorities): Vec<(T::SessionKey, u64)>;

        // xbitcoin
        config(network_id): u32;
        config(genesis): (btc_chain::BlockHeader, u32);
        config(params_info): xbitcoin::Params;

        // multisig
        config(multisig_init_info): (Vec<(T::AccountId, bool)>, u32);

        build(|storage: &mut runtime_primitives::StorageOverlay, _: &mut runtime_primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            use parity_codec::{Encode, KeyedVec};
            use runtime_io::with_externalities;
            use substrate_primitives::Blake2Hasher;
            use runtime_support::{StorageMap, StorageValue};
            use runtime_primitives::{StorageOverlay, traits::{Zero, As}};
            use xaccounts::{TrusteeEntity, TrusteeIntentionProps};
            use xassets::{ChainT, AssetType, Token, Chain, Asset};
            use btc_chain::BlockHeader;
            use xbitcoin::BlockHeaderInfo;
            use xspot::CurrencyPair;

            // grandpa
            let auth_count = config.authorities.len() as u32;
            config.authorities.iter().enumerate().for_each(|(i, v)| {
                storage.insert((i as u32).to_keyed_vec(
                    fg_primitives::well_known_keys::AUTHORITY_PREFIX),
                    v.encode()
                );
            });

            storage.insert(
                fg_primitives::well_known_keys::AUTHORITY_COUNT.to_vec(),
                auth_count.encode(),
            );

            let s = storage.clone().build_storage().unwrap().0;
            let mut init: runtime_io::TestExternalities<Blake2Hasher> = s.into();

            with_externalities(&mut init, || {

                // xassets
                let chainx: Token = <xassets::Module<T> as ChainT>::TOKEN.to_vec();

                let pcx = Asset::new(
                    chainx,
                    config.pcx.0.clone(),
                    Chain::ChainX,
                    config.pcx.1,
                    config.pcx.2.clone(),
                )
                .unwrap();

                xassets::Module::<T>::bootstrap_register_asset(pcx, true, false, Zero::zero()).unwrap();

                // init for asset_list
                for (asset, is_online, is_psedu_intention, init_list) in config.asset_list.iter() {
                    let token = asset.token();
                    xassets::Module::<T>::bootstrap_register_asset(asset.clone(), *is_online, *is_psedu_intention, Zero::zero()).unwrap();

                    for (accountid, value) in init_list {
                        let value: T::Balance = As::sa(*value);
                        let total_free_token = xassets::Module::<T>::total_asset_balance(&token, AssetType::Free);
                        let free_token = xassets::Module::<T>::free_balance(&accountid, &token);
                        xassets::Module::<T>::bootstrap_set_total_asset_balance(&token, AssetType::Free, total_free_token + value);
                        // not create account
                        xassets::Module::<T>::bootstrap_set_asset_balance(&accountid, &token, AssetType::Free, free_token + value);
                    }
                }

                // xbitcoin
                // xbitcoin should be initialized earlier than xstaking due to the network_id.
                let (header, number): (BlockHeader, u32) = config.genesis.clone();
                if config.network_id == 0 && number % config.params_info.retargeting_interval() != 0 {
                    panic!("the blocknumber[{:}] should start from a changed difficulty block", number);
                }
                let genesis = BlockHeaderInfo {
                    header: header,
                    height: number,
                    confirmed: true,
                    txid: [].to_vec(),
                };
                let genesis_header = genesis.header.hash();

                let mut hashes = Vec::new();
                hashes.push(genesis_header.clone());
                <xbitcoin::BlockHeightFor<T>>::insert(&genesis.height, hashes);
                <xbitcoin::BlockHeaderFor<T>>::insert(&genesis_header, genesis);
                <xbitcoin::BestIndex<T>>::put(genesis_header);
                <xbitcoin::NetworkId<T>>::put(config.network_id);

                // xstaking
                let pcx = xassets::Module::<T>::TOKEN.to_vec();
                for (intention, value, name, url) in config.intentions.clone().into_iter() {
                    let _ = xstaking::Module::<T>::bootstrap_register(&intention, name);

                    let _ = <xassets::Module<T>>::pcx_issue(&intention, value);

                    let _ = <xassets::Module<T>>::move_balance(
                        &pcx,
                        &intention,
                        xassets::AssetType::Free,
                        &intention,
                        xassets::AssetType::ReservedStaking,
                        value,
                    );

                    xstaking::Module::<T>::bootstrap_refresh(&intention, Some(url), Some(true), None, None);
                    xstaking::Module::<T>::bootstrap_update_vote_weight(&intention, &intention, value, true);

                    <xstaking::StakeWeight<T>>::insert(&intention, value);
                }

                let mut trustees = Vec::new();
                for (i, hot_entity, cold_entity) in config.trustee_intentions.clone().into_iter() {
                    trustees.push(i.clone());
                    <xaccounts::TrusteeIntentionPropertiesOf<T>>::insert(
                        &(i, xassets::Chain::Bitcoin),
                        TrusteeIntentionProps {
                            about: b"".to_vec(),
                            hot_entity: TrusteeEntity::Bitcoin(hot_entity),
                            cold_entity: TrusteeEntity::Bitcoin(cold_entity),
                        }
                    );
                }
                trustees.sort();
                <xaccounts::TrusteeIntentions<T>>::put(trustees);

                let _ = xbitcoin::Module::<T>::update_trustee_addr();

                // xtokens
                for (token, value_of) in config.endowed_users.iter() {
                    for (who, value) in value_of {
                        xtokens::Module::<T>::bootstrap_update_vote_weight(who, token, *value, true);
                    }
                }

                // xspot
                for (first, second, precision, unit, price, status) in config.pair_list.iter() {
                    let _ = xspot::Module::<T>::add_trading_pair(
                        CurrencyPair(first.clone(), second.clone()),
                        *precision,
                        *unit,
                        *price,
                        *status
                    );
                }

                // xmultisig
                let required_num = config.multisig_init_info.1;
                let init_accounts = config.multisig_init_info.0.clone();
                let (team_addr, council_addr)= xmultisig::Module::<T>::deploy_in_genesis(init_accounts, required_num);
                // set to related place
                xstaking::TeamAddress::<T>::put(team_addr);
                xstaking::CouncilAddress::<T>::put(council_addr);
            });

            let init: StorageOverlay = init.into();
            storage.extend(init);
        });
    }
}
