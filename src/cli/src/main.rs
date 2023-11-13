#![deny(
  unsafe_code,
  // reason = "Let's just not do it"
)]
#![deny(
  clippy::unwrap_used,
  clippy::expect_used,
  clippy::panic,
  clippy::unreachable,
  // reason = "We have to handle errors properly"
)]

mod config;
mod process;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let config = config::Manager::new()?;
  let services = service::Container::new(config.values_async().await);
  let processes = process::Container::new(config, services);

  processes.spawn().await;
  processes.join().await;

  Ok(())
}
