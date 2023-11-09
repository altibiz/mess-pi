use futures::future::join_all;

use crate::{config, service::*};

// TODO: set timeout?

pub struct Process {
  config: config::Manager,
  services: super::Services,
}

impl super::Process for Process {
  fn new(config: config::Manager, services: super::Services) -> Self {
    Self { config, services }
  }
}

#[async_trait::async_trait]
impl super::Recurring for Process {
  async fn execute(&self) -> anyhow::Result<()> {
    let addresses = self.services.network.scan().await;

    let config = self.config.reload_async().await?;

    join_all(
      join_all(
        addresses
          .into_iter()
          .map(modbus::Destination::r#for)
          .flatten()
          .map(|destination| self.r#match(&config, destination)),
      )
      .await
      .into_iter()
      .flatten()
      .map(|r#match| async move {
        match self.services.db.get_device(r#match.id).await {
          Ok(None) => {
            self
              .services
              .db
              .insert_device(db::Device {
                id: r#match.id,
                kind: r#match.kind,
                status: db::DeviceStatus::Healthy,
                address: db::to_network(r#match.destination.address),
                slave: r#match.destination.slave.map(|slave| slave as i32),
              })
              .await;
          }
          _ => {}
        };
      }),
    )
    .await;

    Ok(())
  }
}

#[derive(Debug, Clone)]
struct DeviceMatch {
  id: String,
  kind: String,
  destination: modbus::Destination,
}

impl Process {
  async fn r#match(
    &self,
    config: &config::Parsed,
    destination: modbus::Destination,
  ) -> impl Iterator<Item = DeviceMatch> {
    join_all(
      join_all(config.modbus.devices.values().map(move |device| {
        self
          .services
          .modbus
          .read_from_destination(destination, device.detect.clone())
      }))
      .await
      .into_iter()
      .zip(config.modbus.devices.values())
      .filter_map(|(registers, device)| match registers {
        Ok(registers) => {
          if registers.into_iter().all(|register| register.matches()) {
            Some(device)
          } else {
            None
          }
        }
        Err(_) => None,
      })
      .map(|device| {
        let ids = device.id.clone();
        let kind = device.kind.clone();
        async move {
          (
            self
              .services
              .modbus
              .read_from_destination(destination, ids)
              .await,
            destination,
            kind,
          )
        }
      }),
    )
    .await
    .into_iter()
    .filter_map(|(ids, destination, kind)| match ids {
      Ok(ids) => Some(DeviceMatch {
        id: ids
          .into_iter()
          .map(|id| id.to_string())
          .fold("".to_owned(), |acc, next| acc + next.as_str()),
        destination,
        kind,
      }),
      Err(_) => None,
    })
  }
}
