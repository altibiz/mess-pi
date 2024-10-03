use futures::future::join_all;
use futures_time::future::FutureExt;

#[allow(unused_imports)]
use crate::{service::*, *};

// TODO: use destination to send request because the device might not be discovered

pub(crate) struct Process {
  #[allow(unused)]
  config: config::Manager,

  #[allow(unused)]
  services: service::Container,
}

impl Process {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self { config, services }
  }
}

impl super::Process for Process {}

#[async_trait::async_trait]
impl process::Recurring for Process {
  #[tracing::instrument(skip(self))]
  async fn execute(&self) -> anyhow::Result<()> {
    let config = self.config.values().await;

    let devices = self.services.db().get_devices().await?;

    let pinged_devices = join_all(
      devices
        .iter()
        .cloned()
        .map(|device| self.ping_device(&config, device)),
    )
    .await;
    let pinged_devices_len = pinged_devices.len();
    let healthy_count = pinged_devices.iter().filter(|pinged| **pinged).count();
    let unreachable_count =
      pinged_devices.iter().filter(|pinged| !**pinged).count();
    tracing::info!(
      "Pinged {:?} devices of which {:?} are healthy and {:?} unreachable",
      pinged_devices_len,
      healthy_count,
      unreachable_count,
    );

    let consolidated_devices = join_all(
      pinged_devices
        .into_iter()
        .zip(devices)
        .map(|(pinged, device)| self.consolidate(&config, device, pinged)),
    )
    .await;
    let consolidated_devices_len = consolidated_devices.len();
    let healthy_count = consolidated_devices
      .iter()
      .filter(|consolidated| {
        matches!(**consolidated, Ok((_, db::DeviceStatus::Healthy)))
      })
      .count();
    let unreachable_count = consolidated_devices
      .iter()
      .filter(|consolidated| {
        matches!(**consolidated, Ok((_, db::DeviceStatus::Unreachable)))
      })
      .count();
    let inactive_count = consolidated_devices
      .iter()
      .filter(|consolidated| {
        matches!(**consolidated, Ok((_, db::DeviceStatus::Inactive)))
      })
      .count();
    let failed_count = consolidated_devices
      .iter()
      .filter(|consolidated| consolidated.is_err())
      .count();

    tracing::info!(
      "Consolidated {:?} D {:?} H {:?} U {:?} I {:?} F",
      consolidated_devices_len,
      healthy_count,
      unreachable_count,
      inactive_count,
      failed_count
    );

    Ok(())
  }
}

impl Process {
  #[tracing::instrument(skip(self, config))]
  async fn ping_device(
    &self,
    config: &config::Values,
    device: db::Device,
  ) -> bool {
    match config
      .modbus
      .devices
      .values()
      .find(|device_config| device_config.kind == device.kind)
    {
      Some(device_config) => {
        match self
          .services
          .modbus()
          .read_from_id(&device.id, device_config.id.clone())
          .timeout(timeout_from_chrono(config.modbus.ping_timeout))
          .await
        {
          Err(error) => {
            tracing::warn!("Getting id timed out {}", error);
            return false;
          }
          Ok(Err(error)) => {
            tracing::warn!("Getting id failed {}", error);
            return false;
          }
          Ok(Ok(id_registers)) => {
            if modbus::make_id(device.kind, id_registers) == device.id {
              tracing::debug!("Id match");
            } else {
              tracing::debug!("Id mismatch");
              return false;
            }
          }
        }
      }
      None => {
        tracing::debug!("Config not found");
        return false;
      }
    }

    true
  }

  #[tracing::instrument(skip(self, config, device), fields(id = ?device.id))]
  async fn consolidate(
    &self,
    config: &config::Values,
    device: db::Device,
    pinged: bool,
  ) -> anyhow::Result<(db::Device, db::DeviceStatus)> {
    let now = chrono::Utc::now();
    let status = if pinged {
      db::DeviceStatus::Healthy
    } else if now.signed_duration_since(device.seen)
      > config.modbus.inactive_timeout
    {
      db::DeviceStatus::Inactive
    } else {
      db::DeviceStatus::Unreachable
    };
    let seen = if pinged { now } else { device.seen };
    let update = device.status != status;
    let remove = (status == db::DeviceStatus::Inactive)
      && (device.status != db::DeviceStatus::Inactive);

    if let Err(error) = self
      .services
      .db()
      .update_device_status(&device.id, status, seen, now)
      .await
    {
      tracing::error!("Failed updating device status {}", error);
      return Err(error.into());
    };

    if remove {
      self.services.modbus().stop_from_id(&device.id).await;
    } else {
      self
        .services
        .modbus()
        .bind(
          device.id.clone(),
          modbus::Destination {
            address: self
              .services
              .network()
              .to_socket(db::to_address(device.address)),
            slave: db::to_slave(device.slave),
          },
        )
        .await;
    }

    if update {
      if let Err(error) = self
        .services
        .db()
        .insert_health(db::Health {
          id: 0,
          source: device.id.clone(),
          timestamp: seen,
          status,
          data: serde_json::Value::Object(serde_json::Map::new()),
        })
        .await
      {
        tracing::error!("Failed inserting health {}", error);
      }
    }

    tracing::debug!("Updated device status and health");

    Ok((device, status))
  }
}

fn timeout_from_chrono(
  timeout: chrono::Duration,
) -> futures_time::time::Duration {
  futures_time::time::Duration::from_millis(timeout.num_milliseconds() as u64)
}
