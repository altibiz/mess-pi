mod daily;
mod discover;
mod health;
mod measure;
mod nightly;
mod ping;
mod poll;
mod push;
mod update;

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use crate::{config, service};

// OPTIMIZE: all processes by removing unnecessary cloning at least
// TODO: on startup run discovery to populate modbus device registry

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

  #[error("Job creation startup failed")]
  JobCreation(JobSchedulerError),

  #[error("Job addition startup failed")]
  JobAddition(JobSchedulerError),

  #[error("Job scheduler startup failed")]
  StartupFailed(JobSchedulerError),

  #[error("Job scheduler shutdown failed")]
  ShutdownFailed(JobSchedulerError),
}

macro_rules! add_job_impl {
  ($self: ident, $config: ident, $scheduler: ident, $name: ident, $startup: expr) => {{
    let config = $self.config.clone();
    let services = $self.services.clone();
    let process = Arc::new(Mutex::new($name::Process::new(config, services)));
    #[allow(clippy::redundant_closure_call)] // NOTE: it gets optimized
    {
      $startup(process.clone()).await;
    }
    match Job::new_async_tz(
      $config.schedule.$name,
      $config.schedule.timezone,
      move |uuid, mut lock| {
        let process = process.clone();
        Box::pin(async move {
          let process = process.clone().lock_owned().await;
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
      },
    ) {
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

macro_rules! add_job {
  ($self: ident, $config: ident, $scheduler: ident, $name: ident) => {
    add_job_impl!($self, $config, $scheduler, $name, |process: Arc<
      Mutex<$name::Process>,
    >| {
      Box::pin(async move {
        let process = process.lock_owned().await;
        tracing::debug!("Created process {}", process.process_name());
      })
    })
  };
}

macro_rules! run_add_job {
  ($self: ident, $config: ident, $scheduler: ident, $name: ident) => {
    add_job_impl!($self, $config, $scheduler, $name, |process: Arc<
      Mutex<$name::Process>,
    >| {
      Box::pin(async move {
        let process = process.lock_owned().await;
        if let Err(error) = process.execute().await {
          tracing::error!(
            "Process execution failed {} for {}",
            error,
            process.process_name()
          );
        }
        tracing::debug!("Created process {}", process.process_name());
      })
    })
  };
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

    run_add_job!(self, config, scheduler, poll);
    run_add_job!(self, config, scheduler, discover);
    run_add_job!(self, config, scheduler, ping);
    run_add_job!(self, config, scheduler, measure);
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
