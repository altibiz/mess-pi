# Services

The application sets up services after setting up logging and configuration.

Services are wrapped with a service container which is used by processes to
access services. This is a convenience to avoid unnecessarily copying services
in the application.

## Network

The network service contains functions to scan modbus devices on the network.

Scanning modbus devices on the network is done by trying to open a configurable
modbus port on all devices in a configurable ip range in a configurable timeout.

## Hardware

The hardware service contains functions that return valuable information about
hardware directly on the Raspberry PI.

This is mostly achieved by reading configurable files on the Linux filesystem.

## Modbus

The modbus service manages communication with modbus devices.

Modbus communication can be broken down into levels of abstraction with their
corresponding modules from bottom to top:

- `encoding` - Contains functions for endianness conversion and packing of
  target modbus registers into target architecture bytes for reading operations
  and the reverse for writing operations.
- `register`, `record`, `batch`, and `span` - Contain functions that pack and
  unpack spans of modbus registers into values. The `register` module contains
  functions that parse spans of modbus registers into target primitives and
  strings or bytes and is used for reading operations. The `record` module
  contains simple structs that represent spans of modbus registers and are used
  to write modbus registers as part of writing operations. The `batch` module
  contains functions that pack multiple spans of modbus registers into a single
  span for read operation optimization. The `span` module is an abstraction on
  the various `register`, `record`, and `batch` module types.
- `connection` - Contains a modbus connection struct that allows for reading and
  writing and, in the case of a disconnect, reconnection to a modbus device.
- `worker` - Contains structs used to schedule reads and writes to a modbus
  server. For maximum data throughput, a worker task is spawned for each modbus
  server on the network.

  This worker takes in requests and tries to respond as fast as possible to all
  of them in a fair manner. This means that in the case of a modbus protocol
  exception, disconnect, timeout or any other kind of error, the worker will
  retry a configurable amount of times in a configurable timeout until it moves
  on to other requests. The worker loops over this process as long as there are
  any requests left to respond to and then it waits asynchronously for new
  requests.

  Requests are split into the following categories:

  - read requests - Requests for read operations which are split into two
    different types:
    - oneoff read requests - Requests to read a list of spans once
    - streaming read requests - Requests to read a list of spans indefinitely,
      cancellable by the requestor
  - write requests - Requests to write a list of spans

- `service` - Contains the modbus service and associated structs. The service
  provides an abstraction for other parts of the codebase by hiding the worker
  implementation. Under the hood, the service manages the lifecycle of workers
  and exposes only functions for binding particular workers to devices via their
  identification string, stopping workers for a particular device via its
  identification string, and request and response forwarding functions.

## Cloud

The cloud services manages communication to the cloud server.

This service is a thin wrapper that forwards push requests and responses to and
from the HTTP client.
