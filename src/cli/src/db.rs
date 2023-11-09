use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::{migrate::Migrator, FromRow, Pool, Postgres, QueryBuilder, Type};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Client {
  pool: Pool<Postgres>,
}

#[derive(Debug, Clone, Type)]
pub enum DeviceStatus {
  /// Normal function
  Healthy,
  /// Still taking measurements even though it is unreachable
  Unreachable,
  /// Not taking measurements and unreachable
  Inactive,
}

#[derive(Debug, Clone, FromRow)]
pub struct Device {
  pub id: String,
  pub status: DeviceStatus,
  pub address: IpAddr,
  pub slave: Option<u8>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Measurement {
  pub id: i64,
  pub source: String,
  pub timestamp: DateTime<Utc>,
  pub data: serde_json::Value,
}

#[derive(Debug, Clone, Type)]
#[sqlx(type_name = "log_kind", rename_all = "lowercase")]
pub enum LogKind {
  Success,
  Failure,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Log {
  pub id: i64,
  pub timestamp: DateTime<Utc>,
  pub last_measurement: i64,
  pub kind: LogKind,
  pub response: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum Error {
  #[error("Sqlx error")]
  Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
pub enum MigrateError {
  #[error("Migration failed")]
  Migration(#[from] sqlx::migrate::MigrateError),
}

impl Client {
  pub fn new(
    timeout: u64,
    ssl: bool,
    domain: String,
    port: Option<u16>,
    user: String,
    password: Option<String>,
    name: String,
  ) -> Self {
    let mut options = sqlx::postgres::PgConnectOptions::new()
      .host(domain.as_str())
      .username(user.as_str())
      .database(name.as_str())
      .options([("statement_timeout", timeout.to_string().as_str())]);

    if let Some(port) = port {
      options = options.port(port);
    }

    if let Some(password) = password {
      options = options.password(password.as_str());
    }

    options = options.ssl_mode(sqlx::postgres::PgSslMode::Disable);
    if ssl {
      options = options.ssl_mode(sqlx::postgres::PgSslMode::Require);
    }

    let pool = sqlx::Pool::connect_lazy_with(options);

    let client = Self { pool };

    client
  }

  #[tracing::instrument(skip(self))]
  pub async fn migrate(&self) -> Result<(), MigrateError> {
    MIGRATOR.run(&self.pool).await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_devices(&self) -> Result<(), Error> {
    let devices = sqlx::query_as!(
      Device,
      r#"
        select id, status, address, slave
        from devices
      "#,
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(devices)
  }

  #[tracing::instrument(skip(self))]
  pub async fn insert_device(&self, device: Device) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        insert into devices (id, status, address, slave)
        values ($1, $2, $3, $4)
      "#,
      device.id,
      device.status as DeviceStatus,
      device.address,
      device.slave
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn delete_device(&self, id: String) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        delete from devices
        where id = $1
      "#,
      device.id,
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn update_device_status(
    &self,
    id: String,
    status: DeviceStatus,
  ) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        update devices
        set status = $1
        where id = $1
      "#,
      device.id,
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub async fn insert_measurements(
    &self,
    measurements: Vec<Measurement>,
  ) -> Result<(), Error> {
    let mut query_builder =
      QueryBuilder::new("insert into measurements (source, timestamp, data)");

    query_builder.push_values(measurements, |mut builder, measurement| {
      builder.push_bind(measurement.source);
      builder.push_bind(measurement.timestamp);
      builder.push_bind(measurement.data);
    });

    let query = query_builder.build();

    query.execute(&self.pool).await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_measurements(
    &self,
    from: i64,
    limit: i64,
  ) -> Result<Vec<Measurement>, Error> {
    #[allow(clippy::panic)]
    let measurements = sqlx::query_as!(
      Measurement,
      r#"
        select id, source, timestamp, data
        from measurements
        where measurements.id > $1 
        limit $2
      "#,
      from,
      limit
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(measurements)
  }

  #[tracing::instrument(skip(self))]
  pub async fn insert_log(&self, log: Log) -> Result<(), Error> {
    #[allow(clippy::panic)]
    sqlx::query!(
      r#"
        insert into logs (timestamp, last_measurement, kind, response)
        values ($1, $2, $3, $4)
      "#,
      log.timestamp,
      log.last_measurement,
      log.kind as LogKind,
      log.response
    )
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  #[tracing::instrument(skip(self))]
  pub async fn get_last_successful_log(&self) -> Result<Option<Log>, Error> {
    #[allow(clippy::panic)]
    let log = sqlx::query_as!(
      Log,
      r#"
        select id, timestamp, last_measurement, kind as "kind: LogKind", response
        from logs
        where logs.kind = 'success'::log_kind
        order by timestamp desc
        limit 1
      "#
    )
    .fetch_optional(&self.pool)
    .await?;

    Ok(log)
  }
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
