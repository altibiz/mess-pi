use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{
  migrate::Migrator, types::ipnetwork::IpNetwork, ConnectOptions, FromRow,
  Pool, Postgres, QueryBuilder, Type,
};
use thiserror::Error;

use crate::*;

// TODO: check if lists are empty before sending requests

#[derive(Debug, Clone)]
pub(crate) struct Service {
  pool: Pool<Postgres>,
}

#[derive(Debug, Copy, Clone, Type, Eq, PartialEq)]
#[sqlx(type_name = "device_status", rename_all = "lowercase")]
pub(crate) enum DeviceStatus {
  Healthy,
  Unreachable,
  Inactive,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct Device {
  pub(crate) id: String,
  pub(crate) kind: String,
  pub(crate) status: DeviceStatus,
  pub(crate) address: IpNetwork,
  pub(crate) seen: DateTime<Utc>,
  pub(crate) pinged: DateTime<Utc>,
  pub(crate) slave: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct Measurement {
  #[allow(unused)]
  pub(crate) id: i64,
  pub(crate) source: String,
  pub(crate) timestamp: DateTime<Utc>,
  pub(crate) data: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct Health {
  #[allow(unused)]
  pub(crate) id: i64,
  pub(crate) source: String,
  pub(crate) timestamp: DateTime<Utc>,
  pub(crate) status: DeviceStatus,
  pub(crate) data: serde_json::Value,
}

#[derive(Debug, Copy, Clone, Type, Eq, PartialEq)]
#[sqlx(type_name = "log_status", rename_all = "lowercase")]
pub(crate) enum LogStatus {
  Success,
  Failure,
}

#[derive(Debug, Copy, Clone, Type, Eq, PartialEq)]
#[sqlx(type_name = "log_kind", rename_all = "lowercase")]
pub(crate) enum LogKind {
  Push,
  Update,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct Log {
  #[allow(unused)]
  pub(crate) id: i64,
  pub(crate) timestamp: DateTime<Utc>,
  pub(crate) last: Option<i64>,
  pub(crate) kind: LogKind,
  pub(crate) status: LogStatus,
  pub(crate) response: serde_json::Value,
}

#[derive(Debug, Error)]
pub(crate) enum Error {
  #[error("Sqlx error")]
  Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
pub(crate) enum MigrateError {
  #[error("Migration failed")]
  Migration(#[from] sqlx::migrate::MigrateError),
}

impl service::Service for Service {
  fn new(config: config::Values) -> Self {
    let mut options = sqlx::postgres::PgConnectOptions::new()
      .host(&config.db.domain)
      .username(&config.db.user)
      .database(&config.db.name)
      .log_slow_statements(
        log::LevelFilter::Warn,
        core::time::Duration::from_secs(5),
      )
      .options([(
        "statement_timeout",
        &config.db.timeout.num_milliseconds().to_string(),
      )]);

    if let Some(port) = config.db.port {
      options = options.port(port);
    }

    if let Some(password) = config.db.password {
      options = options.password(password.as_str());
    }

    options = options.ssl_mode(sqlx::postgres::PgSslMode::Disable);
    if config.db.ssl {
      options = options.ssl_mode(sqlx::postgres::PgSslMode::Require);
    }

    let pool = sqlx::Pool::connect_lazy_with(options);

    Self { pool }
  }
}

impl Service {
  #[tracing::instrument(skip(self))]
  pub(crate) async fn migrate(&self) -> Result<(), MigrateError> {
    let mut migration_result = MIGRATOR.run(&self.pool).await;
    let mut migration_retries = 0usize;
    while matches!(migration_result, Err(_)) && migration_retries < 100 {
      migration_result = MIGRATOR.run(&self.pool).await;
      migration_retries = {
        #[allow(clippy::unwrap_used)] // NOTE: it will never pass 100
        let migration_retries = migration_retries.checked_add(1usize).unwrap();
        migration_retries
      };
      tracing::warn!("Migration unsuccsessful - retrying in 1 second...");
      tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    migration_result?;

    tracing::info!("Migration ran successfully");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn get_devices(&self) -> Result<Vec<Device>, Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    let devices = sqlx::query_as!(
      Device,
      r#"
        select id, kind, status as "status: DeviceStatus", seen, pinged, address, slave
        from devices
      "#,
    )
    .fetch_all(&self.pool)
    .await?;

    tracing::trace!("Fetched {:?} devices", devices.len());

    Ok(devices)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn get_device(
    &self,
    id: &str,
  ) -> Result<Option<Device>, Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    let device = sqlx::query_as!(
      Device,
      r#"
        select id, kind, status as "status: DeviceStatus", seen, pinged, address, slave
        from devices
        where id = $1
      "#,
      id
    )
    .fetch_optional(&self.pool)
    .await?;

    tracing::trace!("Fetched device");

    Ok(device)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn insert_device(
    &self,
    device: Device,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        insert into devices (id, kind, status, seen, pinged, address, slave)
        values ($1, $2, $3, $4, $5, $6, $7)
      "#,
      device.id,
      device.kind,
      device.status as DeviceStatus,
      device.seen,
      device.pinged,
      device.address,
      device.slave
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!("Inserted device");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn delete_device(&self, id: &str) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        delete from devices
        where id = $1
      "#,
      id,
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!("Deleted device");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn update_device_status(
    &self,
    id: &str,
    status: DeviceStatus,
    seen: DateTime<Utc>,
    pinged: DateTime<Utc>,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        update devices
        set status = $2::device_status, seen = $3, pinged = $4
        where id = $1
      "#,
      id,
      status as DeviceStatus,
      seen,
      pinged
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!("Updated device status");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn update_device_destination(
    &self,
    id: &str,
    address: IpNetwork,
    slave: Option<i32>,
    seen: DateTime<Utc>,
    pinged: DateTime<Utc>,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        update devices
        set address = $2, slave = $3, seen = $4, pinged = $5
        where id = $1
      "#,
      id,
      address,
      slave,
      seen,
      pinged
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!("Updated device destination");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn insert_measurement(
    &self,
    measurement: Measurement,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        insert into measurements (source, timestamp, data)
        values ($1, $2, $3)
      "#,
      measurement.source,
      measurement.timestamp,
      measurement.data
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!("Inserted measurement");

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub(crate) async fn insert_measurements(
    &self,
    measurements: Vec<Measurement>,
  ) -> Result<(), Error> {
    QueryBuilder::new("insert into measurements (source, timestamp, data)")
      .push_values(measurements, |mut binder, measurement| {
        binder
          .push_bind(measurement.source)
          .push_bind(measurement.timestamp)
          .push_bind(measurement.data);
      })
      .build()
      .execute(&self.pool)
      .await?;

    tracing::trace!("Inserted measurements");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn get_measurements(
    &self,
    from: i64,
    limit: i64,
  ) -> Result<Vec<Measurement>, Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    let measurements = sqlx::query_as!(
      Measurement,
      r#"
        select id, source, timestamp, data
        from measurements
        where measurements.id > $1
        order by measurements.id asc
        limit $2
      "#,
      from,
      limit
    )
    .fetch_all(&self.pool)
    .await?;

    tracing::trace!("Fetched {:?} measurements", measurements.len());

    Ok(measurements)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn insert_health(
    &self,
    health: Health,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        insert into health (source, timestamp, status, data)
        values ($1, $2, $3, $4)
      "#,
      health.source,
      health.timestamp,
      health.status as DeviceStatus,
      health.data
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!(
      "Inserted health for {:?} at {:?}",
      health.source,
      health.timestamp
    );

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(count = healths.len()))]
  pub(crate) async fn insert_healths(
    &self,
    healths: Vec<Health>,
  ) -> Result<(), Error> {
    QueryBuilder::new("insert into health (source, timestamp, status, data)")
      .push_values(healths, |mut binder, health| {
        binder
          .push_bind(health.source)
          .push_bind(health.timestamp)
          .push_bind(health.status)
          .push_bind(health.data);
      })
      .build()
      .execute(&self.pool)
      .await?;

    tracing::trace!("Inserted healths");

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn get_health(
    &self,
    from: i64,
    limit: i64,
  ) -> Result<Vec<Health>, Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    let healths = sqlx::query_as!(
      Health,
      r#"
        select id, source, timestamp, status as "status: DeviceStatus", data
        from health
        where health.id > $1
        limit $2
      "#,
      from,
      limit
    )
    .fetch_all(&self.pool)
    .await?;

    tracing::trace!("Fetched {:?} healths", healths.len());

    Ok(healths)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn insert_log(&self, log: Log) -> Result<(), Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    sqlx::query!(
      r#"
        insert into logs (timestamp, last, status, kind, response)
        values ($1, $2, $3, $4, $5)
      "#,
      log.timestamp,
      log.last,
      log.status as LogStatus,
      log.kind as LogKind,
      log.response
    )
    .execute(&self.pool)
    .await?;

    tracing::trace!(
      "Inserted {:?} {:?} log at {:?}",
      log.status,
      log.kind,
      log.timestamp
    );

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn get_last_successful_push_log(
    &self,
  ) -> Result<Option<Log>, Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    let log = sqlx::query_as!(
      Log,
      r#"
        select id, timestamp, last, kind as "kind: LogKind", status as "status: LogStatus", response
        from logs
        where status = 'success'::log_status and kind = 'push'::log_kind and last is not null
        order by timestamp desc
        limit 1
      "#
    )
    .fetch_optional(&self.pool)
    .await?;

    tracing::trace!(
      "Fetched last successful push log at {:?}",
      log.as_ref().map(|log| log.timestamp)
    );

    Ok(log)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn get_last_successful_update_log(
    &self,
  ) -> Result<Option<Log>, Error> {
    #[allow(clippy::panic)] // NOTE: sqlx thing
    let log = sqlx::query_as!(
      Log,
      r#"
        select id, timestamp, last, kind as "kind: LogKind", status as "status: LogStatus", response
        from logs
        where status = 'success'::log_status and kind = 'update'::log_kind and last is not null
        order by timestamp desc
        limit 1
      "#
    )
    .fetch_optional(&self.pool)
    .await?;

    tracing::trace!(
      "Fetched last successful update log at {:?}",
      log.as_ref().map(|log| log.timestamp)
    );

    Ok(log)
  }
}

pub(crate) fn to_db_address(address: IpAddr) -> IpNetwork {
  #[allow(clippy::unwrap_used)] // NOTE: 24 is valid for ipv4
  IpNetwork::new(address, 24).unwrap()
}

pub(crate) fn to_db_slave(slave: Option<u8>) -> Option<i32> {
  slave.map(|slave| slave as i32)
}

pub(crate) fn to_address(db_address: IpNetwork) -> IpAddr {
  db_address.ip()
}

pub(crate) fn to_slave(db_slave: Option<i32>) -> Option<u8> {
  db_slave.map(|slave| slave as u8)
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
