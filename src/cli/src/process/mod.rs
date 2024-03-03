mod daily;
mod discover;
mod health;
mod measure;
mod nightly;
mod ping;
mod push;
mod update;

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use crate::{config, service};

// OPTIMIZE: all processes by removing unnecessary cloning at least

pub(crate) trait Process {
  fn process_name(&self) -> &'static str {
    std::any::type_name::<Self>()
  }
}

#[async_trait::async_trait]
pub(crate) trait Recurring: Process {
  async fn execute(&self) -> anyhow::Result<()>;
}

pub(crate) struct Container {
  config: config::Manager,
  services: service::Container,
  scheduler: Arc<Mutex<Option<JobScheduler>>>,
}

#[derive(Debug, Error)]
pub(crate) enum ContainerError {
  #[error("Job scheduler creation failed")]
  JobSchedulerCreation(JobSchedulerError),

  #[error("Job creation stratup failed")]
  JobCreation(JobSchedulerError),

  #[error("Job addition stratup failed")]
  JobAddition(JobSchedulerError),

  #[error("Job scheduler stratup failed")]
  StartupFailed(JobSchedulerError),

  #[error("Job scheduler shutdown failed")]
  ShutdownFailed(JobSchedulerError),
}

macro_rules! add_job {
  ($self: ident, $config: ident, $scheduler: ident, $name: ident) => {{
    let config = $self.config.clone();
    let services = $self.services.clone();
    match Job::new_async($config.schedule.$name, move |uuid, mut lock| {
      let config = config.clone();
      let services = services.clone();
      let process = $name::Process::new(config, services);
      tracing::debug!("Created process {}", process.process_name());
      Box::pin(async move {
        tracing::debug!("Starting execution of {}", process.process_name());
        match lock.next_tick_for_job(uuid).await {
          Ok(Some(_)) => {
            if let Err(error) = process.execute().await {
              tracing::error!(
                "Process execution failed {} for {}",
                error,
                process.process_name()
              );
            }
          }
          _ => println!("Could not get next tick for 7s job"),
        }
      })
    }) {
      Ok(job) => {
        if let Err(error) = $scheduler.add(job).await {
          return Err(ContainerError::JobAddition(error));
        }
      }
      Err(error) => {
        return Err(ContainerError::JobCreation(error));
      }
    };
  }};
}

impl Container {
  pub(crate) fn new(
    config: config::Manager,
    services: service::Container,
  ) -> Self {
    Self {
      config,
      services,
      scheduler: Arc::new(Mutex::new(None)),
    }
  }

  pub(crate) async fn startup(&self) -> Result<(), ContainerError> {
    let config = self.config.values().await;
    let scheduler = match JobScheduler::new().await {
      Ok(scheduler) => scheduler,
      Err(error) => {
        return Err(ContainerError::JobSchedulerCreation(error));
      }
    };

    add_job!(self, config, scheduler, discover);
    add_job!(self, config, scheduler, ping);
    add_job!(self, config, scheduler, measure);
    add_job!(self, config, scheduler, push);
    add_job!(self, config, scheduler, update);
    add_job!(self, config, scheduler, health);
    add_job!(self, config, scheduler, daily);
    add_job!(self, config, scheduler, nightly);

    if let Err(error) = scheduler.start().await {
      return Err(ContainerError::StartupFailed(error));
    }

    {
      let mut scheduler_mutex = self.scheduler.clone().lock_owned().await;
      *scheduler_mutex = Some(scheduler);
    }

    Ok(())
  }

  pub(crate) async fn shutdown(&self) -> Result<(), ContainerError> {
    let mut scheduler = self.scheduler.clone().lock_owned().await;
    if let Some(scheduler) = &mut *scheduler {
      if let Err(error) = scheduler.shutdown().await {
        return Err(ContainerError::ShutdownFailed(error));
      }
    }
    *scheduler = None;

    Ok(())
  }
}
