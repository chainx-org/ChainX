// Copyright 2018 Parity Technologies (UK) Ltd.
// Copyright 2018-2019 Chainpool.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

#![warn(unused_extern_crates)]

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};

use client::LongestChain;
use consensus::{import_queue, start_aura, AuraImportQueue, NothingExtra, SlotDuration};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
use inherents::InherentDataProviders;
use network::construct_simple_protocol;
use sr_primitives::generic::BlockId;
use sr_primitives::traits::ProvideRuntimeApi;
use substrate_primitives::{ed25519, Pair as PairT};
use substrate_service::{
    construct_service_factory, FactoryFullConfiguration, FullBackend, FullClient, FullComponents,
    FullExecutor, LightBackend, LightClient, LightComponents, LightExecutor, TaskExecutor,
};
use transaction_pool::txpool::Pool as TransactionPool;

use chainx_primitives::{AccountId, Block};
use chainx_runtime::{GenesisConfig, RuntimeApi};
use runtime_api::xsession_api::XSessionApi;
use substrate_service::TelemetryOnConnect;

type XSystemInherentDataProvider = xsystem::InherentDataProvider;

static mut VALIDATOR_NAME: Option<String> = None;

pub fn set_validator_name(name: String) {
    unsafe {
        VALIDATOR_NAME = Some(name);
    }
}

fn get_validator_name() -> Option<String> {
    unsafe { VALIDATOR_NAME.clone() }
}

construct_simple_protocol! {
    /// Demo protocol attachment for substrate.
    pub struct ChainXProtocol where Block = Block { }
}

/// Node specific configuration
pub struct NodeConfig<F: substrate_service::ServiceFactory> {
    /// grandpa connection to import block
    // FIXME #1134 rather than putting this on the config, let's have an actual intermediate setup state
    pub grandpa_import_setup: Option<(
        Arc<grandpa::BlockImportForService<F>>,
        grandpa::LinkHalfForService<F>,
    )>,
    inherent_data_providers: InherentDataProviders,
}

impl<F> Default for NodeConfig<F>
where
    F: substrate_service::ServiceFactory,
{
    fn default() -> NodeConfig<F> {
        NodeConfig {
            grandpa_import_setup: None,
            inherent_data_providers: InherentDataProviders::new(),
        }
    }
}

construct_service_factory! {
    struct Factory {
        Block = Block,
        RuntimeApi = RuntimeApi,
        NetworkProtocol = ChainXProtocol { |config| Ok(ChainXProtocol::new()) },
        RuntimeDispatch = chainx_executor::Executor,
        FullTransactionPoolApi = transaction_pool::ChainApi<client::Client<FullBackend<Self>, FullExecutor<Self>, Block, RuntimeApi>, Block>
            { |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
        LightTransactionPoolApi = transaction_pool::ChainApi<client::Client<LightBackend<Self>, LightExecutor<Self>, Block, RuntimeApi>, Block>
            { |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
        Genesis = GenesisConfig,
        Configuration = NodeConfig<Self>,
        FullService = FullComponents<Self>
            { |config: FactoryFullConfiguration<Self>, executor: TaskExecutor|
                FullComponents::<Factory>::new(config, executor) },
        AuthoritySetup = {
            |mut service: Self::FullService, executor: TaskExecutor, local_key: Option<Arc<ed25519::Pair>>| {
                let (block_import, link_half) = service.config.custom.grandpa_import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

                if let Some(ref key) = local_key {  //--key
                    info!("Using authority key {:?}", key.public());
                    let proposer = Arc::new(substrate_basic_authorship::ProposerFactory {
                        client: service.client(),
                        transaction_pool: service.transaction_pool(),
                        inherents_pool: service.inherents_pool(),
                    });

                    let client = service.client();
                    let accountid_from_localkey: AccountId = key.public();
                    info!("Using authority key: {:?}, accountid is: {:?}", key.public(), accountid_from_localkey);
                    // use validator name to get accountid and sessionkey from runtime storage
                    let name = get_validator_name().expect("must get validator name is AUTHORITY mode");
                    let best_hash = client.info()?.chain.best_hash;
                    let ret = client
                        .runtime_api()
                        .pubkeys_for_validator_name(&BlockId::Hash(best_hash), name.as_bytes().to_vec())
                        .expect("access runtime data error");

                    let producer = if let Some((accountid, sessionkey_option)) = ret {
                            // check, only print warning log
                            if accountid != accountid_from_localkey {
                                if let Some(sessionkey) = sessionkey_option {
                                    let sessionkey: AccountId = sessionkey.into();
                                    if sessionkey != accountid_from_localkey {
                                        warn!("the sessionkey is not equal to local_key, sessionkey:[{:?}], local_key:[{:?}]", sessionkey, accountid_from_localkey);
                                    }
                                } else {
                                    warn!("the accountid is not equal to local_key, accountid:[{:?}], local_key:[{:?}]", accountid, accountid_from_localkey);
                                }
                            }
                            // anyway, return accountid as producer
                            accountid
                        } else {
                            // do not get accountid from local state database, use localkey as producer
                            warn!("validator name[{:}] is not in current state, use --key|keystore's pri to pub as producer", name);
                            accountid_from_localkey
                        };

                    // set blockproducer for accountid
                    service.config.custom.inherent_data_providers
                        .register_provider(XSystemInherentDataProvider::new(name.as_bytes().to_vec())).expect("blockproducer set err; qed");

                    let client = service.client();
                    executor.spawn(start_aura(
                        SlotDuration::get_or_compute(&*client)?,
                        key.clone(),
                        client,
                        service.select_chain(),
                        block_import.clone(),
                        proposer,
                        service.network(),
                        service.on_exit(),
                        service.config.custom.inherent_data_providers.clone(),
                        service.config.force_authoring,
                    )?);

                    info!("Running Grandpa session as Authority {}", key.public());
                }

                let local_key = if service.config.disable_grandpa {
                    None
                } else {
                    local_key
                };
                let config = grandpa::Config {
                    local_key,
                    // FIXME #1578 make this available through chainspec
                    gossip_duration: Duration::from_millis(3333),
                    justification_period: 4096,
                    name: Some(service.config.name.clone())
                };

                match config.local_key {
                    None => {
                        executor.spawn(grandpa::run_grandpa_observer(
                            config,
                            link_half,
                            service.network(),
                            service.on_exit(),
                        )?);
                    },
                    Some(_) => {
                        let telemetry_on_connect = TelemetryOnConnect {
                          on_exit: Box::new(service.on_exit()),
                          telemetry_connection_sinks: service.telemetry_on_connect_stream(),
                          executor: &executor,
                        };
                        let grandpa_config = grandpa::GrandpaParams {
                          config: config,
                          link: link_half,
                          network: service.network(),
                          inherent_data_providers: service.config.custom.inherent_data_providers.clone(),
                          on_exit: service.on_exit(),
                          telemetry_on_connect: Some(telemetry_on_connect),
                        };
                        executor.spawn(grandpa::run_grandpa_voter(grandpa_config)?);
                    },
                }

                Ok(service)
            }
        },
        LightService = LightComponents<Self>
            { |config, executor| <LightComponents<Factory>>::new(config, executor) },
        FullImportQueue = AuraImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self> , client: Arc<FullClient<Self>>, select_chain: Self::SelectChain| {
                let slot_duration = SlotDuration::get_or_compute(&*client)?;
                let (block_import, link_half) =
                    grandpa::block_import::<_, _, _, RuntimeApi, FullClient<Self>, _>(
                        client.clone(), client.clone(), select_chain
                    )?;
                let block_import = Arc::new(block_import);
                let justification_import = block_import.clone();

                config.custom.grandpa_import_setup = Some((block_import.clone(), link_half));

                import_queue::<_, _, _, ed25519::Pair>(
                    slot_duration,
                    block_import,
                    Some(justification_import),
                    None,
                    None,
                    client,
                    NothingExtra,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
            }},
        LightImportQueue = AuraImportQueue<Self::Block>
            { |config: &FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>| {
                let fetch_checker = client.backend().blockchain().fetcher()
                    .upgrade()
                    .map(|fetcher| fetcher.checker().clone())
                    .ok_or_else(|| "Trying to start light import queue without active fetch checker")?;
                let block_import = grandpa::light_block_import::<_, _, _, RuntimeApi, LightClient<Self>>(
                    client.clone(), Arc::new(fetch_checker), client.clone()
                )?;
                let block_import = Arc::new(block_import);
                let finality_proof_import = block_import.clone();
                let finality_proof_request_builder = finality_proof_import.create_finality_proof_request_builder();

                import_queue::<_, _, _, ed25519::Pair>(
                    SlotDuration::get_or_compute(&*client)?,
                    block_import,
                    None,
                    Some(finality_proof_import),
                    Some(finality_proof_request_builder),
                    client,
                    NothingExtra,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
            }},
        SelectChain = LongestChain<FullBackend<Self>, Self::Block>
            { |config: &FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>| {
                Ok(LongestChain::new(
                    client.backend().clone(),
                    client.import_lock()
                ))
            }
        },
        FinalityProofProvider = { |client: Arc<FullClient<Self>>| {
            Ok(Some(Arc::new(GrandpaFinalityProofProvider::new(client.clone(), client)) as _))
        }},
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "rhd")]
    fn test_sync() {
        use client::{BlockOrigin, ImportBlock};
        use {service_test, Factory};

        let alice: Arc<ed25519::Pair> = Arc::new(Keyring::Alice.into());
        let bob: Arc<ed25519::Pair> = Arc::new(Keyring::Bob.into());
        let validators = vec![alice.public().0.into(), bob.public().0.into()];
        let keys: Vec<&ed25519::Pair> = vec![&*alice, &*bob];
        let dummy_runtime = ::tokio::runtime::Runtime::new().unwrap();
        let block_factory = |service: &<Factory as service::ServiceFactory>::FullService| {
            let block_id = BlockId::number(service.client().info().unwrap().chain.best_number);
            let parent_header = service.client().header(&block_id).unwrap().unwrap();
            let consensus_net = ConsensusNetwork::new(service.network(), service.client().clone());
            let proposer_factory = consensus::ProposerFactory {
                client: service.client().clone(),
                transaction_pool: service.transaction_pool().clone(),
                network: consensus_net,
                force_delay: 0,
                handle: dummy_runtime.executor(),
            };
            let (proposer, _, _) = proposer_factory
                .init(&parent_header, &validators, alice.clone())
                .unwrap();
            let block = proposer.propose().expect("Error making test block");
            ImportBlock {
                origin: BlockOrigin::File,
                justification: Vec::new(),
                internal_justification: Vec::new(),
                finalized: true,
                body: Some(block.extrinsics),
                header: block.header,
                auxiliary: Vec::new(),
            }
        };
        let extrinsic_factory = |service: &<Factory as service::ServiceFactory>::FullService| {
            let payload = (
                0,
                Call::Balances(BalancesCall::transfer(
                    RawAddress::Id(bob.public().0.into()),
                    69.into(),
                )),
                Era::immortal(),
                service.client().genesis_hash(),
            );
            let signature = alice.sign(&payload.encode()).into();
            let id = alice.public().0.into();
            let xt = UncheckedExtrinsic {
                signature: Some((RawAddress::Id(id), signature, payload.0, Era::immortal())),
                function: payload.1,
            }
            .encode();
            let v: Vec<u8> = Decode::decode(&mut xt.as_slice()).unwrap();
            OpaqueExtrinsic(v)
        };
        service_test::sync::<Factory, _, _>(
            chain_spec::integration_test_config(),
            block_factory,
            extrinsic_factory,
        );
    }

}
