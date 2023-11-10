use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

use either::Either;
use futures::Stream;
use futures_time::future::FutureExt;
use tokio::sync::Mutex;

use super::connection::*;
use super::span::{SimpleSpan, Span};

// TODO: inspect errors to terminate/tune

// TODO: optimize
// 1. fix notes
// 4. use Arc slices instead of Vecs
// 6. try spinning

pub type Response = Vec<super::connection::Response>;

#[derive(Debug, thiserror::Error)]
pub enum SendError {
  #[error("Failed to connect")]
  FailedToConnect(#[from] ConnectError),

  #[error("Channel was disconnected before the request could be finished")]
  ChannelDisconnected(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
  #[error("Channel was disconnected before the request could be finished")]
  ChannelDisconnected(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum TerminateError {
  #[error("Channel was disconnected before the request could be finished")]
  ChannelDisconnected(anyhow::Error),
}

type TaskHandle = tokio::task::JoinHandle<()>;

#[derive(Debug, Clone)]
pub struct Worker {
  sender: RequestSender,
  handle: Arc<Mutex<Option<TaskHandle>>>,
  termination_timeout: futures_time::time::Duration,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct SimpleRequest {
  destination: Destination,
  spans: Vec<SimpleSpan>,
}

impl Worker {
  pub fn new(
    initial_params: Params,
    termination_timeout: chrono::Duration,
  ) -> Self {
    let (sender, receiver) = flume::unbounded();
    let task = Task::new(initial_params, receiver);
    let handle = tokio::spawn(task.execute());
    Self {
      sender,
      handle: Arc::new(Mutex::new(Some(handle))),
      termination_timeout: futures_time::time::Duration::from_millis(
        termination_timeout.num_milliseconds() as u64,
      ),
    }
  }
}

impl Worker {
  pub async fn send<TSpan: Span, TIntoIterator: IntoIterator<Item = TSpan>>(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<Response, SendError> {
    let (sender, receiver) = flume::bounded(1);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Carrier(Carrier::new(
        destination,
        spans,
        RequestKind::Oneshot,
        sender,
      )))
      .await
    {
      return Err(SendError::ChannelDisconnected(error.into()));
    };
    let response = match receiver.recv_async().await {
      Ok(response) => response,
      Err(error) => return Err(SendError::ChannelDisconnected(error.into())),
    }?;

    Ok(response)
  }

  pub async fn stream<
    TSpan: Span,
    TIntoIterator: IntoIterator<Item = TSpan>,
  >(
    &self,
    destination: Destination,
    spans: TIntoIterator,
  ) -> Result<
    impl Stream<Item = Result<Response, SendError>> + Send + Sync,
    StreamError,
  > {
    // NOTE: check 1024 is okay
    let (sender, receiver) = flume::bounded(1024);
    if let Err(error) = self
      .sender
      .send_async(TaskRequest::Carrier(Carrier::new(
        destination,
        spans,
        RequestKind::Stream,
        sender,
      )))
      .await
    {
      return Err(StreamError::ChannelDisconnected(error.into()));
    };
    let stream = receiver.into_stream();
    Ok(stream)
  }

  pub async fn terminate(&self) -> Result<(), TerminateError> {
    let result = self.sender.send_async(TaskRequest::Terminate).await;

    let handle = {
      let mut handle = self.handle.clone().lock_owned().await;
      std::mem::replace::<Option<TaskHandle>>(&mut *handle, None)
    };
    if let Some(handle) = handle {
      let abort_handle = handle.abort_handle();
      if let Err(_) =
        flatten_result(handle.timeout(self.termination_timeout).await)
      {
        abort_handle.abort();
      }
    }

    result.map_err(|error| TerminateError::ChannelDisconnected(error.into()))
  }
}

#[derive(Clone, Debug)]
enum RequestKind {
  Oneshot,
  Stream,
}

type SimpleSpans = Vec<SimpleSpan>;

#[derive(Clone, Debug)]
struct Carrier {
  destination: Destination,
  spans: SimpleSpans,
  kind: RequestKind,
  sender: ResponseSender,
}

#[derive(Clone, Debug)]
enum TaskRequest {
  Carrier(Carrier),
  Terminate,
}

impl Carrier {
  fn new<TSpan: Span, TIntoIterator: IntoIterator<Item = TSpan>>(
    destination: Destination,
    spans: TIntoIterator,
    kind: RequestKind,
    sender: ResponseSender,
  ) -> Self {
    Self {
      destination,
      spans: spans
        .into_iter()
        .map(|span| SimpleSpan {
          address: span.address(),
          quantity: span.quantity(),
        })
        .collect::<Vec<_>>(),
      kind,
      sender,
    }
  }
}

type ResponseSender = flume::Sender<Result<Response, SendError>>;
type ResponseReceiver = flume::Receiver<Result<Response, SendError>>;
type RequestSender = flume::Sender<TaskRequest>;
type RequestReceiver = flume::Receiver<TaskRequest>;

type Partial = Vec<Option<super::connection::Response>>;
type Id = uuid::Uuid;

#[derive(Debug, Clone)]
struct Storage {
  id: Id,
  sender: ResponseSender,
  destination: Destination,
  spans: SimpleSpans,
  partial: Partial,
}

#[derive(Debug)]
struct Task {
  connections: HashMap<Destination, Connection>,
  receiver: RequestReceiver,
  oneshots: Vec<Storage>,
  streams: Vec<Storage>,
  params: Params,
  terminate: bool,
}

impl Task {
  pub fn new(params: Params, receiver: RequestReceiver) -> Self {
    Self {
      connections: HashMap::new(),
      receiver,
      oneshots: Vec::new(),
      streams: Vec::new(),
      params,
      terminate: false,
    }
  }

  pub async fn execute(mut self) {
    loop {
      if self.oneshots.is_empty() && self.streams.is_empty() {
        if let Err(error) = self.recv_async_new_request().await {
          match error {
            flume::RecvError::Disconnected => return,
          }
        }
      }

      loop {
        if let Err(error) = self.try_recv_new_request() {
          match error {
            flume::TryRecvError::Empty => break,
            flume::TryRecvError::Disconnected => return,
          }
        }
      }

      let mut metrics = Metrics::new();

      let mut oneshots_to_remove = Vec::new();
      for index in 0..self.oneshots.len() {
        let oneshot = self.oneshots.index(index);
        let connection = match Self::attempt_connection(
          &mut self.connections,
          oneshot,
        )
        .await
        {
          ConnectionAttempt::Existing(connection) => connection,
          ConnectionAttempt::New(connection) => self
            .connections
            .entry(oneshot.destination)
            .or_insert(connection),
          ConnectionAttempt::Fail => {
            oneshots_to_remove.push(oneshot.id);
            continue;
          }
        };

        match Self::read(oneshot, self.params, &mut metrics, connection).await {
          Either::Left(partial) => {
            self.oneshots.index_mut(index).partial = partial
          }
          Either::Right(response) => {
            if let Err(error) = oneshot.sender.try_send(Ok(response)) {
              tracing::debug! {
                %error,
                "Failed sending oneshot response to {:?}",
                oneshot.destination
              }
            }

            oneshots_to_remove.push(oneshot.id);
          }
        };
      }
      self.oneshots.retain(|oneshot| {
        !oneshots_to_remove.iter().any(|id| *id == oneshot.id)
      });

      if self.terminate {
        if !self.streams.is_empty() {
          self.streams = Vec::new();
        }
      } else {
        let mut streams_to_remove = Vec::new();
        for index in 0..self.streams.len() {
          let stream = self.streams.index(index);
          let connection =
            match Self::attempt_connection(&mut self.connections, stream).await
            {
              ConnectionAttempt::Existing(connection) => connection,
              ConnectionAttempt::New(connection) => self
                .connections
                .entry(stream.destination)
                .or_insert(connection),
              ConnectionAttempt::Fail => {
                oneshots_to_remove.push(stream.id);
                continue;
              }
            };

          match Self::read(stream, self.params, &mut metrics, connection).await
          {
            Either::Left(partial) => {
              self.streams.index_mut(index).partial = partial;
            }
            Either::Right(response) => {
              match stream.sender.try_send(Ok(response)) {
                Ok(()) => {
                  self.streams.index_mut(index).partial =
                    vec![None; stream.spans.len()];
                }
                Err(_) => {
                  streams_to_remove.push(stream.id);
                }
              }
            }
          };
        }
        self.streams.retain(|stream| {
          !streams_to_remove.iter().any(|id| *id == stream.id)
        });
      }

      self.tune(metrics);
    }
  }

  fn try_recv_new_request(&mut self) -> Result<(), flume::TryRecvError> {
    match self.receiver.try_recv()? {
      TaskRequest::Carrier(carrier) => self.add_new_request(carrier),
      TaskRequest::Terminate => {
        self.terminate = true;
      }
    }
    Ok(())
  }

  async fn recv_async_new_request(&mut self) -> Result<(), flume::RecvError> {
    match self.receiver.recv_async().await? {
      TaskRequest::Carrier(carrier) => self.add_new_request(carrier),
      TaskRequest::Terminate => {
        self.terminate = true;
      }
    }
    Ok(())
  }

  fn add_new_request(&mut self, carrier: Carrier) {
    let Carrier {
      destination,
      spans,
      kind,
      sender,
    } = carrier;
    let spans_len = spans.len();
    let storage = Storage {
      id: Id::new_v4(),
      sender,
      destination,
      spans,
      partial: vec![None; spans_len],
    };

    match kind {
      RequestKind::Oneshot => self.oneshots.push(storage),
      RequestKind::Stream => self.oneshots.push(storage),
    };
  }
}

enum ConnectionAttempt<'a> {
  Existing(&'a mut Connection),
  New(Connection),
  Fail,
}

impl Task {
  async fn attempt_connection<'a>(
    connections: &'a mut HashMap<Destination, Connection>,
    storage: &Storage,
  ) -> ConnectionAttempt<'a> {
    match connections.get_mut(&storage.destination) {
      Some(connection) => ConnectionAttempt::Existing(connection),
      None => match Connection::connect(storage.destination).await {
        Ok(connection) => ConnectionAttempt::New(connection),
        Err(error) => {
          if let Err(error) = storage.sender.try_send(Err(error.into())) {
            tracing::debug! {
              %error,
              "Failed sending connection fail from worker task to {:?}",
              storage.destination
            }
          }

          ConnectionAttempt::Fail
        }
      },
    }
  }
}

impl Task {
  // NOTE: remove the copying here
  async fn read(
    storage: &Storage,
    params: Params,
    metrics: &mut Metrics,
    connection: &mut Connection,
  ) -> Either<Partial, Response> {
    let partial = {
      let mut data = Vec::new();
      for (span, partial) in
        storage.spans.iter().cloned().zip(storage.partial.iter())
      {
        let read = match partial {
          Some(partial) => Some(partial.clone()),
          None => match (*connection).parameterized_read(span, params).await {
            Ok(read) => Some(read),
            Err(mut errors) => {
              metrics
                .errors
                .entry(storage.destination)
                .or_insert_with(|| Vec::new())
                .append(&mut errors);
              None
            }
          },
        };

        data.push(read);
      }

      data
    };

    if partial.iter().all(|x| x.is_some()) {
      Either::Right(
        partial
          .iter()
          .cloned()
          .filter_map(std::convert::identity)
          .collect::<Vec<_>>(),
      )
    } else {
      Either::Left(partial)
    }
  }
}

#[derive(Debug)]
struct Metrics {
  errors: HashMap<Destination, Vec<ReadError>>,
}

impl Metrics {
  fn new() -> Self {
    Self {
      errors: HashMap::new(),
    }
  }
}

impl Task {
  fn tune(&mut self, metrics: Metrics) {
    dbg!(metrics);
  }
}

fn flatten_result<
  T,
  E1: std::error::Error + Send + Sync,
  E2: std::error::Error + Send + Sync,
>(
  result: Result<Result<T, E1>, E2>,
) -> Result<T, anyhow::Error> {
  Ok(result??)
}
