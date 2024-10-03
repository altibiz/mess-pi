use std::net::{IpAddr, SocketAddr};

use ipnet::IpAddrRange;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct Service {
  ip_range: IpAddrRange,
  timeout: std::time::Duration,
  modbus_port: u16,
}

impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    Self {
      ip_range: config.network.ip_range,
      timeout: std::time::Duration::from_millis(
        config.network.timeout.num_milliseconds() as u64,
      ),
      modbus_port: config.network.modbus_port,
    }
  }
}

impl Service {
  #[tracing::instrument(skip(self))]
  pub(crate) async fn scan_modbus(&self) -> Vec<SocketAddr> {
    let timeout = self.timeout;
    let mut matched_ips = Vec::new();
    let ip_scans = self
      .ip_range
      .map(|ip| {
        let socket_address = self.to_socket(ip);
        (
          socket_address,
          tokio::spawn(async move {
            let timeout = tokio::time::timeout(
              timeout,
              TcpStream::connect(&socket_address),
            )
            .await;
            matches!(timeout, Ok(Ok(_)))
          }),
        )
      })
      .collect::<Vec<(SocketAddr, JoinHandle<bool>)>>();

    tracing::trace!("Matching {:?}", self.ip_range);
    for (ip, scan) in ip_scans {
      if let Ok(true) = scan.await {
        matched_ips.push(ip)
      }
    }
    tracing::trace!("Found {:?} ips", matched_ips.len());

    matched_ips
  }

  pub(crate) fn to_socket(&self, ip: IpAddr) -> SocketAddr {
    SocketAddr::new(ip, self.modbus_port)
  }
}
