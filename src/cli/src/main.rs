#![deny(
  unsafe_code,
  // reason = "Let's just not do it"
)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable,
  clippy::arithmetic_side_effects
  // reason = "We have to handle errors properly"
)]
#![deny(
  clippy::dbg_macro,
  // reason = "Use tracing instead"
)]

mod config;
mod process;
mod service;

use std::fmt::Debug;

use futures_time::future::FutureExt;
use tracing_subscriber::{
  layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

// TODO: configurable timeouts

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
  let format_layer = tracing_subscriber::fmt::layer();
  let (filter_layer, filter_handle) =
    tracing_subscriber::reload::Layer::new(build_tracing_filter("info")?);
  tracing_subscriber::registry()
    .with(filter_layer)
    .with(format_layer)
    .try_init()?;

  let manager = config::Manager::new().await?; // NITPICK: handle this more appropriately
  let config = manager.values().await;

  let log_level = config.log_level.to_string();
  filter_handle.modify(move |filter| {
    #[allow(clippy::unwrap_used)] // NOTE: static and env doesn't change
    let new_filter = build_tracing_filter(log_level.as_str()).unwrap();
    *filter = new_filter;
  })?;

  let id = config.cloud.id.clone();
  tracing::info!("Starting {id}");

  let services = service::Container::new(config.clone());
  let processes = process::Container::new(manager.clone(), services.clone());

  services
    .db()
    .migrate()
    .timeout(futures_time::time::Duration::from_millis(60_000))
    .await??;

  processes.startup().await?;

  if let Err(error) = tokio::signal::ctrl_c().await {
    tracing::error!("Failed waiting for ctrlc signal {}", error);
  };
  if let Err(error) = processes
    .shutdown()
    .timeout(futures_time::time::Duration::from_millis(60_000))
    .await
  {
    tracing::error!("Timed out shutting down processes {}", error);
  }

  Ok(())
}

fn build_tracing_filter(level: &str) -> anyhow::Result<EnvFilter> {
  Ok(
    tracing_subscriber::EnvFilter::builder()
      .with_default_directive(tracing::level_filters::LevelFilter::WARN.into())
      .with_env_var("PIDGEON_LOG")
      .from_env()?
      .add_directive(format!("pidgeon={level}").parse()?),
  )
}
