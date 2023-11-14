use std::net::SocketAddr;

use futures_time::future::FutureExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_modbus::{client::Context, prelude::Reader, Slave};

use super::span::SimpleSpan;

// TODO: tracing

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub(crate) struct Destination {
  pub(crate) address: SocketAddr,
  pub(crate) slave: Option<u8>,
}

impl Destination {
  pub(crate) fn r#for(
    address: SocketAddr,
  ) -> impl Iterator<Item = Destination> {
    (Slave::min_device().0..Slave::max_device().0)
      .map(move |slave| Destination {
        address,
        slave: Some(slave),
      })
      .chain(std::iter::once(Destination {
        address,
        slave: None,
      }))
  }
}

pub(crate) type Response = Vec<u16>;

#[derive(Debug)]
pub(crate) struct Connection {
  destination: Destination,
  ctx: Context,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectError {
  #[error("Failed to connect")]
  Connect(#[from] std::io::Error),

  #[error("Wrong slave number")]
  Slave,
}

impl Connection {
  pub(crate) async fn connect(
    destination: Destination,
  ) -> Result<Self, ConnectError> {
    match destination.slave {
      Some(slave) => Self::connect_slave(destination.address, slave).await,
      None => Self::connect_standalone(destination.address).await,
    }
  }

  pub(crate) async fn connect_standalone(
    socket: SocketAddr,
  ) -> Result<Self, ConnectError> {
    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::tcp::attach(stream);
    Ok(Self {
      destination: Destination {
        address: socket,
        slave: None,
      },
      ctx,
    })
  }

  pub(crate) async fn connect_slave(
    socket: SocketAddr,
    slave: u8,
  ) -> Result<Self, ConnectError> {
    if Slave(slave) < Slave::min_device() || Slave(slave) > Slave::max_device()
    {
      return Err(ConnectError::Slave);
    }

    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::rtu::attach_slave(stream, Slave(slave));
    Ok(Self {
      destination: Destination {
        address: socket,
        slave: Some(slave),
      },
      ctx,
    })
  }

  #[inline]
  pub(crate) fn socket(&self) -> SocketAddr {
    self.destination.address
  }

  #[inline]
  pub(crate) fn slave(&self) -> Option<u8> {
    self.destination.slave
  }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Params {
  timeout: futures_time::time::Duration,
  backoff: tokio::time::Duration,
  retries: u32,
}

impl Params {
  pub(crate) fn new(
    timeout: chrono::Duration,
    backoff: chrono::Duration,
    retries: u32,
  ) -> Self {
    let timeout = timeout_from_chrono(timeout);
    let backoff = backoff_from_chrono(backoff);
    Self {
      timeout,
      backoff,
      retries,
    }
  }

  #[inline]
  pub(crate) fn timeout(self) -> chrono::Duration {
    timeout_to_chrono(self.timeout)
  }

  #[inline]
  pub(crate) fn backoff(self) -> chrono::Duration {
    backoff_to_chrono(self.backoff)
  }

  #[inline]
  pub(crate) fn retries(self) -> u32 {
    self.retries
  }
}

#[derive(Debug, Error)]
pub(crate) enum ReadError {
  #[error("Failed connecting")]
  Connection(std::io::Error),

  #[error("Connection timed out")]
  Timeout(std::io::Error),
}

impl Connection {
  pub(crate) async fn parameterized_read(
    &mut self,
    span: SimpleSpan,
    params: Params,
  ) -> Result<Response, Vec<ReadError>> {
    let timeout = params.timeout;
    let backoff = params.backoff;
    let retries = params.retries;
    let mut errors = Vec::new();
    let mut retried = 0;
    let mut response = None;
    while response.is_none() && retried != retries {
      tokio::time::sleep(backoff).await;
      match self.simple_read_impl(span, timeout).await {
        Ok(data) => response = Some(data),
        Err(error) => errors.push(error),
      };
      retried += 1;
    }

    response.ok_or(errors)
  }

  pub(crate) async fn simple_read(
    &mut self,
    span: SimpleSpan,
    timeout: chrono::Duration,
  ) -> Result<Response, ReadError> {
    let response = self
      .simple_read_impl(span, timeout_from_chrono(timeout))
      .await?;

    Ok(response)
  }

  async fn simple_read_impl(
    &mut self,
    span: SimpleSpan,
    timeout: futures_time::time::Duration,
  ) -> Result<Response, ReadError> {
    let response = match self
      .ctx
      .read_holding_registers(span.address, span.quantity)
      .timeout(timeout)
      .await
    {
      Ok(timeout_response) => match timeout_response {
        Ok(response) => Ok(response),
        Err(timeout_error) => Err(ReadError::Timeout(timeout_error)),
      },
      Err(connection_error) => Err(ReadError::Connection(connection_error)),
    }?;

    Ok(response)
  }
}

fn timeout_to_chrono(
  timeout: futures_time::time::Duration,
) -> chrono::Duration {
  chrono::Duration::milliseconds(timeout.as_millis() as i64)
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}

fn backoff_to_chrono(backoff: tokio::time::Duration) -> chrono::Duration {
  chrono::Duration::milliseconds(backoff.as_millis() as i64)
}

fn backoff_from_chrono(backoff: chrono::Duration) -> tokio::time::Duration {
  tokio::time::Duration::from_millis(backoff.num_milliseconds() as u64)
}
