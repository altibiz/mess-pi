use std::time::Duration;

use thiserror::Error;

use crate::{
  config::{ConfigManager, ConfigManagerError},
  services::{ServiceError, Services},
};

#[derive(Debug)]
pub struct Runtime {
  scan_interval: Duration,
  pull_interval: Duration,
  push_interval: Duration,
  r#async: tokio::runtime::Runtime,
  services: Services,
}

struct Interval {
  token: tokio_util::sync::CancellationToken,
  handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
  #[error("Logging setup error")]
  LogSetup,

  #[error("Async runtime error")]
  AsyncRuntime(#[from] std::io::Error),

  #[error("Config manager error")]
  ConfigManager(#[from] ConfigManagerError),

  #[error("Service error")]
  Service(#[from] ServiceError),
}

macro_rules! interval {
  ($rt:ident,$handler:ident,$duration:expr) => {{
    let services = $rt.services.clone();
    let token = tokio_util::sync::CancellationToken::new();
    let child_token = token.child_token();
    let duration = $duration.clone();
    let handle = $rt.r#async.spawn(async move {
      let mut interval = tokio::time::interval(duration);

      loop {
        tokio::select! {
            _ = child_token.cancelled() => {
                return;
            },
            _ = async {
                if let Err(error) = services.$handler().await {
                    tracing::error! { %error, "interval handler failed" };
                }

                interval.tick().await;
            } => {

            }
        }
      }
    });

    Interval { token, handle }
  }};
}

macro_rules! kill_intervals {
    [$($interval:expr),*] => {
        $(
            $interval.token.cancel();
        )*

        $(
            if let Err(error) = $interval.handle.await {
                tracing::error! { %error, "Interval exited with error" }
            }
        )*
    };
}

impl Runtime {
  pub fn new() -> Result<Self, RuntimeError> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
      return Err(RuntimeError::LogSetup);
    };

    let config_manager = ConfigManager::new()?;
    let config = config_manager.config()?;

    let services = Services::new(config_manager)?;

    let r#async = tokio::runtime::Builder::new_multi_thread()
      .worker_threads(4)
      .enable_all()
      .build()?;

    let runtime = Self {
      scan_interval: Duration::from_millis(config.runtime.scan_interval),
      pull_interval: Duration::from_millis(config.runtime.pull_interval),
      push_interval: Duration::from_millis(config.runtime.push_interval),
      services,
      r#async,
    };

    Ok(runtime)
  }

  pub fn start(&self) -> Result<(), RuntimeError> {
    self.r#async.block_on(async { self.start_async().await })
  }

  async fn start_async(&self) -> Result<(), RuntimeError> {
    self.services.on_setup().await?;

    let scan = interval!(self, on_scan, self.scan_interval);
    let pull = interval!(self, on_pull, self.pull_interval);
    let push = interval!(self, on_push, self.push_interval);

    if let Err(error) = tokio::signal::ctrl_c().await {
      tracing::error! { %error, "Failed waiting for Ctrl+C" }
    }

    kill_intervals![scan, pull, push];

    Ok(())
  }
}
