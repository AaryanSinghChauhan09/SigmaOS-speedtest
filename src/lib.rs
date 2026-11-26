//! SigmaOS speedtest — network latency and throughput measurement.
//!
//! Measures TCP connect latency and download throughput against a target
//! host. Uses only std::net — no external deps, safe code only.

#![forbid(unsafe_code)]

use serde::Serialize;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Errors produced by this component.
#[derive(Debug, thiserror::Error)]
pub enum SpeedtestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid address: {0}")]
    InvalidAddr(String),
    #[error("timeout connecting to {0}")]
    Timeout(String),
    #[error("no address resolved for {0}")]
    NoAddr(String),
}

/// Result of a single ping (TCP connect).
#[derive(Debug, Clone, Serialize)]
pub struct PingResult {
    pub host: String,
    pub port: u16,
    pub latency_ms: f64,
    pub ok: bool,
}

/// Throughput measurement result.
#[derive(Debug, Clone, Serialize)]
pub struct ThroughputResult {
    pub host: String,
    pub port: u16,
    pub bytes_downloaded: u64,
    pub duration_ms: u64,
    pub mbps: f64,
}

/// Measure TCP connect latency to host:port.
pub fn ping(host: &str, port: u16, timeout: Duration) -> Result<PingResult, SpeedtestError> {
    let addr_str = format!("{host}:{port}");
    let addrs: Vec<_> = addr_str
        .to_socket_addrs()
        .map_err(|_| SpeedtestError::InvalidAddr(addr_str.clone()))?
        .collect();
    let addr = addrs
        .first()
        .ok_or_else(|| SpeedtestError::NoAddr(addr_str.clone()))?;

    let start = Instant::now();
    let stream = TcpStream::connect_timeout(addr, timeout)
        .map_err(|_| SpeedtestError::Timeout(addr_str.clone()))?;
    let latency = start.elapsed();
    drop(stream);

    Ok(PingResult {
        host: host.to_string(),
        port,
        latency_ms: latency.as_secs_f64() * 1000.0,
        ok: true,
    })
}

/// Measure download throughput by reading up to `max_bytes` from a TCP
/// stream that the server keeps writing to (e.g. HTTP response body).
///
/// `request` is written to the socket first (e.g. an HTTP GET).
pub fn download(
    host: &str,
    port: u16,
    request: &[u8],
    max_bytes: u64,
    timeout: Duration,
) -> Result<ThroughputResult, SpeedtestError> {
    let addr_str = format!("{host}:{port}");
    let addr = addr_str
        .to_socket_addrs()
        .map_err(|_| SpeedtestError::InvalidAddr(addr_str.clone()))?
        .next()
        .ok_or_else(|| SpeedtestError::NoAddr(addr_str.clone()))?;

    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|_| SpeedtestError::Timeout(addr_str.clone()))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.write_all(request)?;

    let start = Instant::now();
    let mut buf = [0u8; 65536];
    let mut total: u64 = 0;
    while total < max_bytes {
        let want = std::cmp::min(buf.len() as u64, max_bytes - total) as usize;
        match stream.read(&mut buf[..want]) {
            Ok(0) => break,
            Ok(n) => total += n as u64,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break
            }
            Err(e) => return Err(SpeedtestError::Io(e)),
        }
    }
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64().max(0.0001);
    let mbps = (total as f64 * 8.0) / (secs * 1_000_000.0);

    Ok(ThroughputResult {
        host: host.to_string(),
        port,
        bytes_downloaded: total,
        duration_ms: elapsed.as_millis() as u64,
        mbps,
    })
}

/// Convenience: ping N times, return min/avg/max latency in ms.
pub fn ping_stats(
    host: &str,
    port: u16,
    count: usize,
    timeout: Duration,
) -> Result<(f64, f64, f64), SpeedtestError> {
    let mut times = Vec::with_capacity(count);
    for _ in 0..count {
        let r = ping(host, port, timeout)?;
        times.push(r.latency_ms);
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = times.first().copied().unwrap_or(0.0);
    let max = times.last().copied().unwrap_or(0.0);
    let avg = times.iter().sum::<f64>() / times.len().max(1) as f64;
    Ok((min, avg, max))
}
