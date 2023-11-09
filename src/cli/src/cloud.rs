use std::{fs, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::{
  header::{HeaderMap, HeaderValue, InvalidHeaderValue},
  Client as HttpClient, Error as HttpError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
  pub device_id: String,
  pub timestamp: DateTime<Utc>,
  pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
  pub device_id: String,
  pub timestamp: DateTime<Utc>,
  pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushRequest {
  timestamp: DateTime<Utc>,
  measurements: Vec<Measurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRequest {
  timestamp: DateTime<Utc>,
  health: Vec<Health>,
}

#[derive(Debug, Clone)]
pub struct Response {
  pub success: bool,
  pub text: String,
}

#[derive(Debug, Clone)]
pub struct Client {
  push_endpoint: String,
  update_endpoint: String,
  http: HttpClient,
}

#[derive(Debug, Error)]
pub enum ConstructionError {
  #[error("HTTP client construction error")]
  HttpError(#[from] HttpError),

  #[error("Invalid header error")]
  InvalidHeader(#[from] InvalidHeaderValue),

  #[error("IO error")]
  IO(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum PushError {
  #[error("HTTP Post error")]
  HttpError(#[from] HttpError),
}

impl Client {
  pub fn new(
    domain: String,
    ssl: bool,
    api_key: Option<String>,
    timeout: u64,
    id: Option<String>,
  ) -> Result<Self, ConstructionError> {
    let id = match id {
      Some(id) => id,
      None => {
        "pidgeon-".to_string()
          + fs::read_to_string("/sys/firmware/devicetree/base/serial-number")?
            .as_str()
      }
    };

    let protocol = if ssl { "https" } else { "http" };

    let push_endpoint = format!("{protocol}://{domain}/push/{id}");
    let update_endpoint = format!("{protocol}://{domain}/update/{id}");

    let mut headers = HeaderMap::new();
    match api_key {
      Some(api_key) => {
        let value = HeaderValue::from_str(api_key.as_str())?;
        headers.insert("X-API-Key", value);
      }
      None => {
        let value = HeaderValue::from_str((id + "-oil-rulz-5000").as_str())?;
        headers.insert("X-API-Key", value);
      }
    };

    let builder = HttpClient::builder()
      .timeout(Duration::from_millis(timeout))
      .default_headers(headers)
      .gzip(true);

    let http = builder.build()?;

    let client = Self {
      push_endpoint,
      update_endpoint,
      http,
    };

    Ok(client)
  }

  #[tracing::instrument(skip_all, fields(count = measurements.len()))]
  pub async fn push(
    &self,
    measurements: Vec<Measurement>,
  ) -> Result<Response, PushError> {
    let request = PushRequest {
      timestamp: chrono::offset::Utc::now(),
      measurements,
    };

    let http_response = self
      .http
      .post(self.push_endpoint.clone())
      .json(&request)
      .send()
      .await;
    if let Err(error) = &http_response {
      tracing::warn! {
        %error,
        "Failed pushing {:?} measurements: connection error",
        request.measurements.len(),
      }
    }
    let http_response = http_response?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    if success {
      tracing::debug! {
        "Successfully pushed {:?} measurements",
        request.measurements.len()
      };
    } else {
      tracing::warn! {
        "Failed pushing {:?} measurements: {:?} {:?}",
        request.measurements.len(),
        status_code,
        text.clone()
      };
    }

    let response = Response { success, text };

    Ok(response)
  }

  #[tracing::instrument(skip_all, fields(count = health.len()))]
  pub async fn update(
    &self,
    health: Vec<Health>,
  ) -> Result<Response, PushError> {
    let request = UpdateRequest {
      timestamp: chrono::offset::Utc::now(),
      health,
    };

    let http_response = self
      .http
      .post(self.update_endpoint.clone())
      .json(&request)
      .send()
      .await;
    if let Err(error) = &http_response {
      tracing::warn! {
        %error,
        "Failed pushing {:?} measurements: connection error",
        request.health.len(),
      }
    }
    let http_response = http_response?;

    let status_code = http_response.status();
    let success = status_code.is_success();
    let text = http_response.text().await?;

    if success {
      tracing::debug! {
        "Successfully updated {:?} health",
        request.health.len()
      };
    } else {
      tracing::warn! {
        "Failed updating {:?} health: {:?} {:?}",
        request.health.len(),
        status_code,
        text.clone()
      };
    }

    let response = Response { success, text };

    Ok(response)
  }
}
