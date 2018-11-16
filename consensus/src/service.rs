// Copyright 2018 Chainpool.

//! Consensus service.
use std::sync::Arc;
use std::thread;
/// Consensus service. A long running service that manages BFT agreement over the network.
///
/// This uses a handle to an underlying thread pool to dispatch heavy work
/// such as candidate verification while performing event-driven work
/// on a local event loop.
use std::time::{Duration, Instant};

use super::{Network, ProposerFactory};
use bft::{self, BftService};
use client::{BlockBody, BlockchainEvents, ChainHead};
use ed25519;
use error;

use futures::prelude::*;
use tokio::executor::current_thread::TaskExecutor as LocalThreadHandle;
use tokio::runtime::current_thread::Runtime as LocalRuntime;
use tokio::runtime::TaskExecutor as ThreadPoolHandle;
use tokio::timer::Interval;

use chainx_api::ChainXApi;
use chainx_primitives::{Block, Header};
use TransactionPool;

const TIMER_DELAY_MS: Duration = Duration::from_millis(3000);
const TIMER_INTERVAL_MS: Duration = Duration::from_millis(300);

// spin up an instance of BFT agreement on the current thread's executor.
// panics if there is no current thread executor.
fn start_bft<F, C>(header: Header, bft_service: Arc<BftService<Block, F, C>>)
where
    F: bft::Environment<Block> + 'static,
    C: bft::BlockImport<Block> + bft::Authorities<Block> + 'static,
    F::Error: ::std::fmt::Debug,
    <F::Proposer as bft::Proposer<Block>>::Error: ::std::fmt::Display + Into<error::Error>,
    <F as bft::Environment<Block>>::Error: ::std::fmt::Display,
{
    let mut handle = LocalThreadHandle::current();
    match bft_service.build_upon(&header) {
        Ok(Some(bft_work)) => {
            if let Err(e) = handle.spawn_local(Box::new(bft_work)) {
                warn!(target: "bft", "Couldn't initialize BFT agreement: {:?}", e);
            }
        }
        Ok(None) => trace!(target: "bft", "Could not start agreement on top of {}", header.hash()),
        Err(e) => warn!(target: "bft", "BFT agreement error: {}", e),
    }
}

/// Consensus service. Starts working when created.
pub struct Service {
    thread: Option<thread::JoinHandle<()>>,
    exit_signal: Option<::exit_future::Signal>,
}

impl Service {
    /// Create and start a new instance.
    pub fn new<A, C, N>(
        client: Arc<C>,
        api: Arc<A>,
        network: N,
        transaction_pool: Arc<TransactionPool<A>>,
        thread_pool: ThreadPoolHandle,
        key: ed25519::Pair,
        block_delay: u64,
    ) -> Service
    where
        A: ChainXApi + Send + Sync + 'static,
        C: BlockchainEvents<Block> + ChainHead<Block> + BlockBody<Block>,
        C: bft::BlockImport<Block> + bft::Authorities<Block> + Send + Sync + 'static,
        N: Network + Send + 'static,
    {
        use super::OfflineTracker;
        use parking_lot::RwLock;

        let (signal, exit) = ::exit_future::signal();
        let thread = thread::spawn(move || {
            let mut runtime = LocalRuntime::new().expect("Could not create local runtime");
            let key = Arc::new(key);

            let factory = ProposerFactory {
                client: api.clone(),
                transaction_pool: transaction_pool.clone(),
                network,
                handle: thread_pool.clone(),
                offline: Arc::new(RwLock::new(OfflineTracker::new())),
                force_delay: block_delay,
            };
            let bft_service = Arc::new(BftService::new(client.clone(), key, factory));

            let notifications = {
                let client = client.clone();
                let bft_service = bft_service.clone();

                client.import_notification_stream().for_each(move |notification| {
                    if notification.is_new_best {
                        trace!(target: "bft", "Attempting to start new consensus round after import notification of {:?}", notification.hash);
                        start_bft(notification.header, bft_service.clone());
                    }
                    Ok(())
                })
            };

            let interval = Interval::new(Instant::now() + TIMER_DELAY_MS, TIMER_INTERVAL_MS);

            let mut prev_best = match client.best_block_header() {
                Ok(header) => header.hash(),
                Err(e) => {
                    warn!(
                        "Cant's start consensus service. Error reading best block header: {:?}",
                        e
                    );
                    return;
                }
            };

            let timed = {
                let c = client.clone();
                let s = bft_service.clone();

                interval
                    .map_err(|e| debug!(target: "bft", "Timer error: {:?}", e))
                    .for_each(move |_| {
                        if let Ok(best_block) = c.best_block_header() {
                            let hash = best_block.hash();

                            if hash == prev_best {
                                trace!(target: "bft", "Starting consensus round after a timeout");
                                start_bft(best_block, s.clone());
                            }
                            prev_best = hash;
                        }
                        Ok(())
                    })
            };

            runtime.spawn(notifications);
            runtime.spawn(timed);

            if let Err(e) = runtime.block_on(exit) {
                debug!("BFT event loop error {:?}", e);
            }
        });
        Service {
            thread: Some(thread),
            exit_signal: Some(signal),
        }
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        if let Some(signal) = self.exit_signal.take() {
            signal.fire();
        }

        if let Some(thread) = self.thread.take() {
            thread.join().expect("The service thread has panicked");
        }
    }
}
