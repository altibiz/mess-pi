use std::{collections::HashMap, str::FromStr};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::service::modbus::{self, RegisterValue};

// NITPICK: optional values here with #[serde(default = ...)]

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Hardware {
  pub(crate) temperature_monitor: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Network {
  pub(crate) timeout: Option<u32>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Db {
  pub(crate) timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MeasurementRegister {
  pub(crate) name: String,
  pub(crate) address: u16,
  pub(crate) kind: RegisterKindStorage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct StringRegisterKind {
  pub(crate) length: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct RawRegisterKind {
  pub(crate) length: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct NumericRegisterKind {
  pub(crate) multiplier: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RegisterKindStorage {
  U16(NumericRegisterKind),
  U32(NumericRegisterKind),
  U64(NumericRegisterKind),
  S16(NumericRegisterKind),
  S32(NumericRegisterKind),
  S64(NumericRegisterKind),
  F32(NumericRegisterKind),
  F64(NumericRegisterKind),
  String(StringRegisterKind),
  Raw(RawRegisterKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DetectRegister {
  pub(crate) address: u16,
  pub(crate) kind: RegisterKindStorage,
  pub(crate) r#match: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IdRegister {
  pub(crate) address: u16,
  pub(crate) kind: RegisterKindStorage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ValueRegister {
  pub(crate) address: u16,
  pub(crate) value: Vec<u16>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Device {
  pub(crate) detect: Vec<DetectRegister>,
  pub(crate) id: Vec<IdRegister>,
  pub(crate) measurement: Vec<MeasurementRegister>,
  pub(crate) configuration: Vec<ValueRegister>,
  pub(crate) daily: Vec<ValueRegister>,
  pub(crate) nightly: Vec<ValueRegister>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Modbus {
  pub(crate) read_timeout: Option<u32>,
  pub(crate) batch_threshold: Option<u16>,
  pub(crate) termination_timeout: Option<u32>,
  pub(crate) congestion_backoff: Option<u32>,
  pub(crate) partial_retries: Option<u32>,
  pub(crate) ping_timeout: Option<u32>,
  pub(crate) tariff_timeout: Option<u32>,
  pub(crate) inactive_timeout: Option<u32>,
  pub(crate) discovery_timeout: Option<u32>,
  pub(crate) devices: HashMap<String, Device>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Cloud {
  pub(crate) timeout: Option<u32>,
  pub(crate) message_limit: Option<i64>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Schedule {
  pub(crate) discover: Option<String>,
  pub(crate) ping: Option<String>,
  pub(crate) measure: Option<String>,
  pub(crate) push: Option<String>,
  pub(crate) update: Option<String>,
  pub(crate) health: Option<String>,
  pub(crate) daily: Option<String>,
  pub(crate) nightly: Option<String>,
  pub(crate) poll: Option<String>,
  pub(crate) timezone: Option<chrono_tz::Tz>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Values {
  pub(crate) log_level: Option<LogLevel>,
  #[serde(default)]
  pub(crate) hardware: Hardware,
  #[serde(default)]
  pub(crate) network: Network,
  #[serde(default)]
  pub(crate) cloud: Cloud,
  #[serde(default)]
  pub(crate) db: Db,
  #[serde(default)]
  pub(crate) modbus: Modbus,
  #[serde(default)]
  pub(crate) schedule: Schedule,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseError {
  #[error("Failed creating project directories")]
  MissingProjectDirs,

  #[error("Failed reading config file")]
  Read(#[from] std::io::Error),

  #[error("Config file is missing an extension")]
  MissingExtension,

  #[error("Config file has invalid extension")]
  InvalidExtension,

  #[error("Failed deserializing config from yaml")]
  DeserializetionYaml(#[from] serde_yaml::Error),

  #[error("Failed deserializing config from toml")]
  DeserializetionToml(#[from] toml::de::Error),

  #[error("Failed deserializing config from json")]
  DeserializetionJson(#[from] serde_json::Error),
}

pub(crate) async fn parse_file(
  location: Option<&str>,
) -> Result<Values, ParseError> {
  let location = match location {
    Some(location) => std::path::PathBuf::from(location),
    None => match directories::ProjectDirs::from("com", "altibiz", "pidgeon") {
      Some(project_dirs) => project_dirs.config_dir().join("config.yaml"),
      None => return Err(ParseError::MissingProjectDirs),
    },
  };

  let values = {
    let raw = tokio::fs::read_to_string(location.clone()).await?;
    match location.extension().and_then(|str| str.to_str()) {
      None => return Err(ParseError::MissingExtension),
      Some("yaml" | "yml") => serde_yaml::from_str::<Values>(raw.as_str())?,
      Some("toml") => toml::from_str::<Values>(raw.as_str())?,
      Some("json") => serde_json::from_str::<Values>(raw.as_str())?,
      Some(_) => return Err(ParseError::InvalidExtension),
    }
  };

  Ok(values)
}

pub(crate) async fn parse_json(json: &str) -> Result<Values, ParseError> {
  let parsed = serde_json::from_str::<Values>(json)?;

  Ok(parsed)
}

pub(crate) fn to_modbus_measurement_register(
  register: MeasurementRegister,
) -> modbus::MeasurementRegister<modbus::RegisterKindStorage> {
  modbus::MeasurementRegister::<modbus::RegisterKindStorage> {
    address: register.address,
    storage: to_modbus_register_kind(register.kind),
    name: register.name,
  }
}

pub(crate) fn to_modbus_detect_register(
  register: DetectRegister,
) -> modbus::DetectRegister<modbus::RegisterKindStorage> {
  modbus::DetectRegister::<modbus::RegisterKindStorage> {
    address: register.address,
    storage: to_modbus_register_kind(register.kind),
    r#match: match regex::Regex::new(register.r#match.as_str()) {
      Ok(regex) => either::Either::Right(regex),
      _ => either::Either::Left(register.r#match),
    },
  }
}

pub(crate) fn to_modbus_id_register(
  register: IdRegister,
) -> modbus::IdRegister<modbus::RegisterKindStorage> {
  modbus::IdRegister::<modbus::RegisterKindStorage> {
    address: register.address,
    storage: to_modbus_register_kind(register.kind),
  }
}

pub(crate) fn to_modbus_value_register(
  register: ValueRegister,
) -> modbus::ValueRegister<modbus::RegisterValueStorage> {
  modbus::ValueRegister::<modbus::RegisterValueStorage> {
    address: register.address,
    storage: modbus::RegisterValueStorage::Raw(RegisterValue::<_> {
      value: register.value,
      timestamp: chrono::Utc::now(),
    }),
  }
}

pub(crate) fn to_modbus_register_kind(
  register: RegisterKindStorage,
) -> modbus::RegisterKindStorage {
  match register {
    RegisterKindStorage::U16(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::U16(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::U32(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::U32(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::U64(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::U64(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::S16(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::S16(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::S32(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::S32(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::S64(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::S64(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::F32(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::F32(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::F64(NumericRegisterKind { multiplier }) => {
      modbus::RegisterKindStorage::F64(modbus::NumericRegisterKind {
        multiplier,
      })
    }
    RegisterKindStorage::String(StringRegisterKind { length }) => {
      modbus::RegisterKindStorage::String(modbus::StringRegisterKind { length })
    }
    RegisterKindStorage::Raw(RawRegisterKind { length }) => {
      modbus::RegisterKindStorage::Raw(modbus::RawRegisterKind { length })
    }
  }
}

pub(crate) fn make_ip_range(start: String, end: String) -> ipnet::IpAddrRange {
  let (start, end) = match (start.parse(), end.parse()) {
    (Ok(start), Ok(end)) => (start, end),
    #[allow(clippy::unwrap_used)] // NOTE: valid ipv4 addresses
    err => {
      tracing::warn!("Invalid IP addresses {:?}", err);
      (
        "192.168.1.0".parse().unwrap(),
        "192.168.1.255".parse().unwrap(),
      )
    }
  };

  ipnet::IpAddrRange::from(ipnet::Ipv4AddrRange::new(start, end))
}

pub(crate) fn milliseconds_to_chrono(milliseconds: u32) -> chrono::Duration {
  chrono::Duration::milliseconds(milliseconds as i64)
}

pub(crate) fn string_to_cron(
  string: &Option<String>,
  default: &str,
) -> cron::Schedule {
  let string = {
    if let Some(string) = string {
      string.as_str()
    } else {
      default
    }
  };

  let result = cron::Schedule::from_str(string);
  if let Ok(result) = result {
    return result;
  }

  let result = cron::Schedule::from_str(default);
  match result {
    Ok(result) => return result,
    Err(error) => {
      tracing::warn! {
        %error,
        "Failed parsing default set schedule string"
      }
    }
  }

  #[allow(clippy::unwrap_used)] // NOTE: this is a valid cron expression
  cron::Schedule::from_str(
    // NOTE: every minute
    // NOTE: sec | min | hour | day of month | month | day of week | year
    "        0     *     *      *              *       *             *",
  )
  .unwrap()
}
