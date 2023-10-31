use std::net::SocketAddr;

use futures_time::future::FutureExt;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_modbus::{client::Context, prelude::Reader, Slave};

use super::span::*;

#[derive(Debug)]
pub struct Connection {
  ctx: Context,
  timeout: futures_time::time::Duration,
  backoff: tokio::time::Duration,
  retries: usize,
}

impl Connection {
  pub async fn connect(
    socket: SocketAddr,
    timeout: futures_time::time::Duration,
    backoff: tokio::time::Duration,
    retries: usize,
  ) -> Result<Self, std::io::Error> {
    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::tcp::attach(stream);
    Ok(Self {
      ctx,
      timeout,
      backoff,
      retries,
    })
  }

  pub async fn connect_slave(
    socket: SocketAddr,
    slave: Slave,
    timeout: futures_time::time::Duration,
    backoff: tokio::time::Duration,
    retries: usize,
  ) -> Result<Self, std::io::Error> {
    let stream = TcpStream::connect(socket).await?;
    let ctx = tokio_modbus::prelude::rtu::attach_slave(stream, slave);
    Ok(Self {
      ctx,
      timeout,
      backoff,
      retries,
    })
  }
}

#[derive(Debug, Error)]
pub enum ConnectionReadError {
  #[error("Failed connecting to device")]
  Connection(#[from] std::io::Error),

  #[error("Failed to parse response")]
  Parse,
}

impl Connection {
  pub async fn read_spans<
    TParsedSpan: Span,
    TUnparsedSpan: UnparsedSpan<TParsedSpan>,
    TIntoIterator,
  >(
    &mut self,
    spans: TIntoIterator,
  ) -> Vec<Result<TParsedSpan, ConnectionReadError>>
  where
    for<'a> &'a TIntoIterator: IntoIterator<Item = &'a TUnparsedSpan>,
  {
    let mut results = Vec::new();
    let backoff = self.backoff;
    for span in spans.into_iter() {
      let parsed = self.read_span(span).await;
      results.push(parsed);
      tokio::time::sleep(backoff).await;
    }
    results
  }

  pub async fn read_span<
    TParsedSpan: Span,
    TUnparsedSpan: UnparsedSpan<TParsedSpan>,
  >(
    &mut self,
    register: &TUnparsedSpan,
  ) -> Result<TParsedSpan, ConnectionReadError> {
    fn flatten_result<T, E1, E2>(
      result: Result<Result<T, E1>, E2>,
    ) -> Result<T, E1>
    where
      E1: From<E2>,
    {
      result?
    }

    let data = {
      let address = register.address();
      let quantity = register.quantity();
      let timeout = self.timeout;
      let backoff = self.backoff;
      let retries = self.retries;
      let mut retried = 0;
      let mut result = flatten_result(
        self
          .ctx
          .read_holding_registers(address, quantity)
          .timeout(timeout)
          .await,
      );
      while result.is_err() && retried != retries {
        tokio::time::sleep(backoff).await;
        result = flatten_result(
          self
            .ctx
            .read_holding_registers(address, quantity)
            .timeout(timeout)
            .await,
        );
        retried = retried + 1;
      }
      result
    }?;
    let parsed = register.parse(data.iter().cloned());
    parsed.ok_or_else(|| ConnectionReadError::Parse)
  }
}
