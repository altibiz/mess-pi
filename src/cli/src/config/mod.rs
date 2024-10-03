mod args;
mod env;
mod file;

use std::{collections::HashMap, fs, sync::Arc};

use ipnet::IpAddrRange;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::service::modbus::{self, RegisterValueStorage};

#[derive(Debug, Clone)]
pub(crate) struct Db {
  pub(crate) timeout: chrono::Duration,
  pub(crate) ssl: bool,
  pub(crate) domain: String,
  pub(crate) port: Option<u16>,
  pub(crate) user: String,
  pub(crate) password: Option<String>,
  pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Network {
  pub(crate) timeout: chrono::Duration,
  pub(crate) ip_range: IpAddrRange,
  pub(crate) modbus_port: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Hardware {
  pub(crate) temperature_monitor: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Cloud {
  pub(crate) timeout: chrono::Duration,
  pub(crate) message_limit: i64,
  pub(crate) ssl: bool,
  pub(crate) domain: String,
  pub(crate) api_key: Option<String>,
  pub(crate) id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Device {
  pub(crate) kind: String,
  pub(crate) id: Vec<modbus::IdRegister<modbus::RegisterKindStorage>>,
  pub(crate) detect: Vec<modbus::DetectRegister<modbus::RegisterKindStorage>>,
  pub(crate) measurement:
    Vec<modbus::MeasurementRegister<modbus::RegisterKindStorage>>,
  pub(crate) configuration: Vec<modbus::ValueRegister<RegisterValueStorage>>,
  pub(crate) daily: Vec<modbus::ValueRegister<RegisterValueStorage>>,
  pub(crate) nightly: Vec<modbus::ValueRegister<RegisterValueStorage>>,
}

#[derive(Debug, Clone)]
pub(crate) struct Modbus {
  pub(crate) read_timeout: chrono::Duration,
  pub(crate) batch_threshold: u16,
  pub(crate) termination_timeout: chrono::Duration,
  pub(crate) congestion_backoff: chrono::Duration,
  pub(crate) partial_retries: u32,
  pub(crate) ping_timeout: chrono::Duration,
  pub(crate) tariff_timeout: chrono::Duration,
  pub(crate) inactive_timeout: chrono::Duration,
  pub(crate) discovery_timeout: chrono::Duration,
  pub(crate) devices: HashMap<String, Device>,
}

#[derive(Debug, Clone)]
pub(crate) struct Schedule {
  pub(crate) discover: cron::Schedule,
  pub(crate) ping: cron::Schedule,
  pub(crate) measure: cron::Schedule,
  pub(crate) push: cron::Schedule,
  pub(crate) update: cron::Schedule,
  pub(crate) health: cron::Schedule,
  pub(crate) daily: cron::Schedule,
  pub(crate) nightly: cron::Schedule,
  pub(crate) poll: cron::Schedule,
  pub(crate) timezone: chrono_tz::Tz,
}

#[derive(Debug, Clone)]
pub(crate) struct Values {
  pub(crate) cloud: Cloud,
  pub(crate) db: Db,
  pub(crate) network: Network,
  pub(crate) modbus: Modbus,
  pub(crate) hardware: Hardware,
  pub(crate) schedule: Schedule,
  pub(crate) log_level: tracing::level_filters::LevelFilter,
}

#[derive(Debug, Clone)]
struct Unparsed {
  from_args: args::Values,
  from_env: env::Values,
  from_file: file::Values,
}

#[derive(Debug, Clone)]
pub(crate) struct Manager {
  lock: Arc<Mutex<Unparsed>>,
}

#[derive(Debug, Error)]
pub(crate) enum ReadError {
  #[error("Failed reading file")]
  FileReadError(#[from] file::ParseError),

  #[error("Failed reading env")]
  EnvReadError(#[from] env::ParseError),
}

#[derive(Debug, Error)]
pub(crate) enum ReloadError {
  #[error("Failed reading file")]
  FileReadError(#[from] file::ParseError),
}

impl Manager {
  pub(crate) async fn new() -> Result<Self, ReadError> {
    let config = Self::read().await?;

    let config_manager = Self {
      lock: Arc::new(Mutex::new(config)),
    };

    Ok(config_manager)
  }

  pub(crate) async fn values(&self) -> Values {
    let config = self.lock.lock().await.clone();

    Self::parse(config)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn reload(&self) -> Values {
    let config = {
      let mut values = self.lock.lock().await;
      let from_file =
        file::parse_file(values.from_args.config.as_deref()).await;
      match from_file {
        Ok(from_file) => values.from_file = from_file,
        Err(error) => {
          tracing::error!("Failed parsing config file {}", error)
        }
      }
      values.clone()
    };

    Self::parse(config)
  }

  #[tracing::instrument(skip(self))]
  pub(crate) async fn reload_json(&self, json: &str) -> Values {
    let config = {
      let mut values = self.lock.lock().await;
      let from_file = file::parse_json(json).await;
      match from_file {
        Ok(from_file) => values.from_file = from_file,
        Err(error) => {
          tracing::error!("Failed parsing config json {}", error)
        }
      }
      values.clone()
    };

    Self::parse(config)
  }

  fn parse(config: Unparsed) -> Values {
    Values {
      log_level: if config.from_args.trace {
        tracing::level_filters::LevelFilter::TRACE
      } else if config.from_args.debug {
        tracing::level_filters::LevelFilter::DEBUG
      } else if let Some(log_level) = config.from_file.log_level {
        match log_level {
          file::LogLevel::Trace => tracing::level_filters::LevelFilter::TRACE,
          file::LogLevel::Debug => tracing::level_filters::LevelFilter::DEBUG,
          file::LogLevel::Info => tracing::level_filters::LevelFilter::INFO,
          file::LogLevel::Warn => tracing::level_filters::LevelFilter::WARN,
          file::LogLevel::Error => tracing::level_filters::LevelFilter::ERROR,
        }
      } else {
        #[cfg(debug_assertions)]
        {
          tracing::level_filters::LevelFilter::DEBUG
        }
        #[cfg(not(debug_assertions))]
        {
          tracing::level_filters::LevelFilter::INFO
        }
      },
      schedule: Schedule {
        discover: file::string_to_cron(
          &config.from_file.schedule.discover,
          "0 */5 * * * * *", // NOTE: every 5 minutes
        ),
        ping: file::string_to_cron(
          &config.from_file.schedule.ping,
          "0 */5 * * * * *", // NOTE: every 5 minutes
        ),
        measure: file::string_to_cron(
          &config.from_file.schedule.measure,
          "0 * * * * * *", // NOTE: every minute
        ),
        push: file::string_to_cron(
          &config.from_file.schedule.push,
          "0 * * * * * *", // NOTE: every minute
        ),
        update: file::string_to_cron(
          &config.from_file.schedule.update,
          "0 * * * * * *", // NOTE: every minute
        ),
        health: file::string_to_cron(
          &config.from_file.schedule.health,
          "0 * * * * * *", // NOTE: every minute
        ),
        daily: file::string_to_cron(
          &config.from_file.schedule.daily,
          "0 0 7 * * * *", // NOTE: at 7:00
        ),
        nightly: file::string_to_cron(
          &config.from_file.schedule.nightly,
          "0 0 21 * * * *", // NOTE: at 21:00
        ),
        poll: file::string_to_cron(
          &config.from_file.schedule.poll,
          "0 * * * * * *", // NOTE: every minute
        ),
        timezone: config.from_file.schedule.timezone.unwrap_or(chrono_tz::UTC),
      },
      hardware: Hardware {
        temperature_monitor: config
          .from_file
          .hardware
          .temperature_monitor
          .unwrap_or("/sys/class/hwmon/hwmon1/temp1_input".to_owned()),
      },
      cloud: Cloud {
        timeout: file::milliseconds_to_chrono(
          config.from_file.cloud.timeout.unwrap_or(30000),
        ),
        message_limit: config.from_file.cloud.message_limit.unwrap_or(50000),
        ssl: config.from_env.cloud.ssl,
        domain: config.from_env.cloud.domain,
        api_key: config.from_env.cloud.api_key,
        id: config.from_env.cloud.id.unwrap_or_else(|| {
          #[allow(clippy::unwrap_used)] // NOTE: it is for sure there on rpi4
          {
            "pidgeon-".to_string()
              + fs::read_to_string(
                "/sys/firmware/devicetree/base/serial-number",
              )
              .unwrap()
              .as_str()
          }
        }),
      },
      db: Db {
        timeout: file::milliseconds_to_chrono(
          config.from_file.db.timeout.unwrap_or(30_000),
        ),
        ssl: config.from_env.db.ssl,
        domain: config.from_env.db.domain,
        port: config
          .from_env
          .db
          .port
          .and_then(|port| port.parse::<u16>().ok()),
        user: config.from_env.db.user,
        password: config.from_env.db.password,
        name: config.from_env.db.name,
      },
      network: Network {
        timeout: file::milliseconds_to_chrono(
          config.from_file.network.timeout.unwrap_or(30_000),
        ),
        ip_range: file::make_ip_range(
          config.from_env.network.ip_range_start,
          config.from_env.network.ip_range_end,
        ),
        modbus_port: config.from_env.network.modbus_port,
      },
      modbus: Modbus {
        read_timeout: file::milliseconds_to_chrono(
          config.from_file.modbus.read_timeout.unwrap_or(100),
        ),
        batch_threshold: config.from_file.modbus.batch_threshold.unwrap_or(4),
        termination_timeout: file::milliseconds_to_chrono(
          config
            .from_file
            .modbus
            .termination_timeout
            .unwrap_or(10_000),
        ),
        congestion_backoff: file::milliseconds_to_chrono(
          config.from_file.modbus.congestion_backoff.unwrap_or(50),
        ),
        partial_retries: config.from_file.modbus.partial_retries.unwrap_or(10),
        ping_timeout: file::milliseconds_to_chrono(
          config.from_file.modbus.ping_timeout.unwrap_or(30_000),
        ),
        tariff_timeout: file::milliseconds_to_chrono(
          config.from_file.modbus.tariff_timeout.unwrap_or(30_000),
        ),
        inactive_timeout: file::milliseconds_to_chrono(
          config
            .from_file
            .modbus
            .inactive_timeout
            .unwrap_or(5 * 60 * 1000),
        ),
        discovery_timeout: file::milliseconds_to_chrono(
          config.from_file.modbus.discovery_timeout.unwrap_or(30_000),
        ),
        devices: config
          .from_file
          .modbus
          .devices
          .into_iter()
          .map(|(kind, device)| {
            (
              kind.clone(),
              Device {
                kind,
                id: device
                  .id
                  .into_iter()
                  .map(file::to_modbus_id_register)
                  .collect(),
                detect: device
                  .detect
                  .into_iter()
                  .map(file::to_modbus_detect_register)
                  .collect(),
                measurement: device
                  .measurement
                  .into_iter()
                  .map(file::to_modbus_measurement_register)
                  .collect(),
                configuration: device
                  .configuration
                  .into_iter()
                  .map(file::to_modbus_value_register)
                  .collect(),
                daily: device
                  .daily
                  .into_iter()
                  .map(file::to_modbus_value_register)
                  .collect(),
                nightly: device
                  .nightly
                  .into_iter()
                  .map(file::to_modbus_value_register)
                  .collect(),
              },
            )
          })
          .collect::<HashMap<_, _>>(),
      },
    }
  }

  async fn read() -> Result<Unparsed, ReadError> {
    let from_args = args::parse();
    let from_env = env::parse()?;
    let from_file = file::parse_file(from_args.config.as_deref()).await?;

    Ok(Unparsed {
      from_args,
      from_env,
      from_file,
    })
  }
}
