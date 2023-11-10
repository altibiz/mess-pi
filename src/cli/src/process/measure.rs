use either::Either;
use futures::{future::try_join_all, TryFutureExt};
use futures_core::Stream;

use crate::{config, service::*};

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
impl super::Background for Process {
  async fn execute(&self) {
    let config = self.config.reload_async().await.unwrap();

    let mut stream_devices = Vec::new();

    loop {
      if let Ok(devices_from_db) = self.get_devices_from_db(config).await {
        devices_from_db;
      }
    }
  }
}

type MeasurementStreamRegisters = Vec<
  Either<
    modbus::IdRegister<modbus::RegisterValue>,
    modbus::MeasurementRegister<modbus::RegisterValue>,
  >,
>;

type BoxedMeasurementStream = Box<
  dyn Stream<Item = Result<MeasurementStreamRegisters, modbus::ServerReadError>>
    + Send
    + Sync,
>;

struct Device {
  id: String,
  kind: String,
  destination: modbus::Destination,
  id_registers: Vec<modbus::IdRegister<modbus::RegisterKind>>,
  measurement_registers: Vec<modbus::MeasurementRegister<modbus::RegisterKind>>,
}

struct StreamDevice {
  device: Device,
  stream: BoxedMeasurementStream,
}

impl Process {
  async fn get_devices_from_db(
    &self,
    config: config::Parsed,
  ) -> anyhow::Result<Vec<Device>> {
    Ok(
      self
        .services
        .db
        .get_devices()
        .await?
        .into_iter()
        .filter_map(|device| {
          config
            .modbus
            .devices
            .values()
            .filter(|device_config| device_config.kind == device.kind)
            .next()
            .map(|config| Device {
              id: device.id,
              kind: device.kind,
              destination: modbus::Destination {
                address: network::to_socket(db::to_ip(device.address)),
                slave: db::to_modbus_slave(device.slave),
              },
              id_registers: config.id.clone(),
              measurement_registers: config.measurement.clone(),
            })
        })
        .collect::<Vec<_>>(),
    )
  }

  async fn merge_devices(
    old_devices: Vec<StreamDevice>,
    new_devices: Vec<Device>,
  ) -> Vec<StreamDevice> {
  }

  async fn make_stream(
    &self,
    device: db::Device,
    config: config::ParsedDevice,
  ) -> anyhow::Result<BoxedMeasurementStream> {
    Ok(Box::new(
      self
        .services
        .modbus
        .stream_from_id(
          &device.id,
          config
            .id
            .into_iter()
            .map(|register| Either::Left(register))
            .chain(
              config
                .measurement
                .into_iter()
                .map(|register| Either::Right(register)),
            )
            .collect::<Vec<_>>(),
        )
        .await?,
    ))
  }

  async fn consolidate(
    &self,
    kind: String,
    id_to_verify: String,
    registers: MeasurementStreamRegisters,
  ) -> Result<(), anyhow::Error> {
    let id_got =
      modbus::make_id(kind, registers.iter().cloned().filter_map(Either::left));

    if id_got != id_to_verify {
      return Err(anyhow::anyhow!(format!(
        "Id register mismatch: expected {id_to_verify} but got {id_got}"
      )));
    }

    self
      .services
      .db
      .insert_measurement(db::Measurement {
        id: 0,
        source: id_got,
        timestamp: chrono::Utc::now(),
        data: modbus::serialize_registers(
          registers.into_iter().filter_map(Either::right),
        ),
      })
      .await?;

    Ok(())
  }
}
