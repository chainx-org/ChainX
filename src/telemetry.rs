// Copyright 2018 chainpool

use substrate_network::{TransactionPool, SyncState, SyncProvider};
use substrate_runtime_primitives::traits::{Header, As};
use substrate_client::BlockchainEvents;
use tel;

use sysinfo::{get_current_pid, ProcessExt, System, SystemExt};
use std::time::{Duration, Instant};
use tokio::runtime::TaskExecutor;
use tokio::prelude::{Future, Stream};
use tokio::timer::Interval;
use ansi_term::Colour;

const TIMER_INTERVAL_MS: u64 = 5000;

pub fn build_telemetry(
    telemetry_url: Option<String>,
    is_authority: bool,
) -> Option<tel::Telemetry> {
    let telemetry = match telemetry_url {
        Some(url) => {
            Some(tel::init_telemetry(tel::TelemetryConfig {
                url: url,
                on_connect: Box::new(move || {
                    telemetry!("system.connected";
                            "name" => "chainx",
                            "implementation" => "chainx",
                            "version" => "0.1",
                            "config" => "",
                            "chain" => "chainx",
                            "authority" => is_authority
                        );
                }),
            }))
        }
        None => None,
    };
    telemetry
}

pub fn run_telemetry(
    network: ::Arc<::chainx_network::NetworkService>,
    client: ::Arc<::client::TClient>,
    _txpool: ::Arc<TransactionPool<::Hash, ::Block>>,
    handle: TaskExecutor,
) {
    let interval = Interval::new(Instant::now(), Duration::from_millis(TIMER_INTERVAL_MS));

    let mut last_number = None;
    let mut sys = System::new();
    let self_pid = get_current_pid();
    let client1 = client.clone();
    let display_notifications = interval.map_err(|e| debug!("Timer error: {:?}", e)).for_each(move |_| {
        let sync_status = network.status();
        if let Ok(best_block) = client1.best_block_header() {
            let hash = best_block.hash();
            let num_peers = sync_status.num_peers;
            let best_number: u64 = best_block.number().as_();
            let speed = move || speed(best_number, last_number);
            let (status, target) =
              match (sync_status.sync.state, sync_status.sync.best_seen_block) {
                (SyncState::Idle, _) => ("Idle".into(), "".into()),
                (SyncState::Downloading, None) => (format!("Syncing{}", speed()), "".into()),
                (SyncState::Downloading, Some(n)) =>
                   (format!("Syncing{}", speed()), format!(", target=#{}", n)),
            };
            last_number = Some(best_number);
            info!(
                target: "substrate",
                "{}{} ({} peers), best: #{} ({})",
                Colour::White.bold().paint(&status),
                target,
                Colour::White.bold().paint(format!("{}", sync_status.num_peers)),
                Colour::White.paint(format!("{}", best_number)),
                hash
            );

            // get cpu usage and memory usage of this process
            let (cpu_usage, memory) = if sys.refresh_process(self_pid) {
                let proc = sys.get_process(self_pid).expect("Above refresh_process succeeds, this should be Some(), qed");
                (proc.cpu_usage(), proc.memory())
            } else { (0.0, 0) };
            telemetry!(
                "system.interval";
                "status" => format!("{}{}", status, target),
                "peers" => num_peers,
                "height" => best_number,
                "best" => ?hash,
                "cpu" => cpu_usage,
                "memory" => memory
            );
        } else {
            warn!("Error getting best block information");
        }
        Ok(())
    });

    let display_block_import = client.import_notification_stream().for_each(|n| {
        info!(target: "substrate", "Imported #{} ({})", n.header.number(), n.hash);
        Ok(())
    });

    /*let display_txpool_import = txpool.import_notification_stream().for_each(move |_| {
        let status = txpool.light_status();
        telemetry!("txpool.import";
                   "mem_usage" => status.mem_usage,
                   "count" => status.transaction_count,
                   "sender" => status.senders);
        Ok(())
    });*/

    let informant_work = display_notifications.join(display_block_import);
    handle.spawn(informant_work.map(|_| ()));
}

fn speed(best_number: u64, last_number: Option<u64>) -> String {
    let speed = match last_number {
        Some(num) => (best_number.saturating_sub(num) * 10_000 / TIMER_INTERVAL_MS) as f64,
        None => 0.0,
    };

    if speed < 1.0 {
        "".into()
    } else {
        format!(" {:4.1} bps", speed / 10.0)
    }
}
