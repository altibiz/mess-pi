#[derive(Debug, Clone)]
pub(crate) struct Cloud {
  pub(crate) ssl: bool,
  pub(crate) domain: String,
  pub(crate) api_key: Option<String>,
  pub(crate) id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Db {
  pub(crate) ssl: bool,
  pub(crate) domain: String,
  pub(crate) port: Option<String>,
  pub(crate) user: String,
  pub(crate) password: Option<String>,
  pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Network {
  pub(crate) ip_range_start: String,
  pub(crate) ip_range_end: String,
  pub(crate) modbus_port: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Values {
  pub(crate) cloud: Cloud,
  pub(crate) db: Db,
  pub(crate) network: Network,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseError {
  #[error("Failed reading env var")]
  EnvVarRead(#[from] std::env::VarError),
}

pub(crate) fn parse() -> Result<Values, ParseError> {
  if let Err(error) = dotenv::dotenv() {
    tracing::warn!("Failed reading dotenv {error}");
  }

  let values = Values {
    cloud: Cloud {
      ssl: std::env::var("PIDGEON_CLOUD_SSL").map_or_else(|_| false, |_| true),
      domain: std::env::var("PIDGEON_CLOUD_DOMAIN")?,
      api_key: std::env::var("PIDGEON_CLOUD_API_KEY").ok(),
      id: std::env::var("PIDGEON_CLOUD_ID").ok(),
    },
    db: Db {
      ssl: std::env::var("PIDGEON_DB_SSL").map_or_else(|_| false, |_| true),
      domain: std::env::var("PIDGEON_DB_DOMAIN")?,
      port: std::env::var("PIDGEON_DB_PORT").ok(),
      user: std::env::var("PIDGEON_DB_USER")?,
      password: std::env::var("PIDGEON_DB_PASSWORD").ok(),
      name: std::env::var("PIDGEON_DB_NAME")?,
    },
    network: Network {
      ip_range_start: std::env::var("PIDGEON_NETWORK_IP_RANGE_START")?,
      ip_range_end: std::env::var("PIDGEON_NETWORK_IP_RANGE_END")?,
      modbus_port: std::env::var("PIDGEON_MODBUS_PORT").map_or_else(
        |_| 502,
        |port| port.as_str().parse::<u16>().unwrap_or(502),
      ),
    },
  };

  Ok(values)
}
