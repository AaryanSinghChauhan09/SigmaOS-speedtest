//! CLI for sigmaos-speedtest.

use clap::{Parser, Subcommand};
use sigmaos_speedtest::{download, ping, ping_stats};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "sigmaos-speedtest", about = "SigmaOS network speed test")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Measure TCP connect latency.
    Ping {
        host: String,
        #[arg(long, default_value_t = 80)]
        port: u16,
        #[arg(long, default_value_t = 4)]
        count: usize,
        /// Timeout per attempt in ms.
        #[arg(long, default_value_t = 3000)]
        timeout_ms: u64,
    },
    /// Measure download throughput (HTTP GET).
    Download {
        host: String,
        #[arg(long, default_value_t = 80)]
        port: u16,
        /// Max bytes to download.
        #[arg(long, default_value_t = 10 * 1024 * 1024)]
        max_bytes: u64,
        #[arg(long, default_value_t = 10000)]
        timeout_ms: u64,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    match Cli::parse().command {
        Commands::Ping {
            host,
            port,
            count,
            timeout_ms,
        } => {
            let timeout = Duration::from_millis(timeout_ms);
            if count <= 1 {
                let r = ping(&host, port, timeout)?;
                println!("{}:{} latency {:.2} ms", host, port, r.latency_ms);
            } else {
                let (min, avg, max) = ping_stats(&host, port, count, timeout)?;
                println!(
                    "{}:{} min {:.2} ms / avg {:.2} ms / max {:.2} ms ({} pings)",
                    host, port, min, avg, max, count
                );
            }
        }
        Commands::Download {
            host,
            port,
            max_bytes,
            timeout_ms,
        } => {
            let req = format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
            let r = download(
                &host,
                port,
                req.as_bytes(),
                max_bytes,
                Duration::from_millis(timeout_ms),
            )?;
            println!(
                "{}:{} {:.2} Mbps ({:.1} MiB in {} ms)",
                host,
                port,
                r.mbps,
                r.bytes_downloaded as f64 / 1024.0 / 1024.0,
                r.duration_ms
            );
        }
    }
    Ok(())
}
