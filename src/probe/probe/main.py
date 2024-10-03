import asyncio
import time
import json
from signal import SIGINT, SIGTERM
from typing import Any, List, Union
from probe.log import log
from probe.client import Client
from probe.loader import Loader
from probe.server import Server
from probe.args import Args
from probe.device import DeviceType
from probe.request import abb_b2x_requests, schneider_iem3xxx_requests, ReadRequest, WriteRequest


async def async_main(args: Args):
  requests = None
  device_type = args.device_type()

  if device_type == DeviceType.abb_b2x:
    requests = abb_b2x_requests
  elif device_type == DeviceType.schneider_iem3xxx:
    requests = schneider_iem3xxx_requests
  else:
    raise ValueError("Unknown device type")

  if args.is_server():
    loader = Loader(
      device_type=device_type,
      id=args.id(),
      config=args.config(),
      measurements=args.measurements(),
    )
    server = Server(
      address=args.address(),
      port=args.port(),
      registers=loader.measurement_to_registers(next(iter(loader))),
    )

    async def update():
      async for measurement in loader:
        log.info((
          "Setting registers from measurement:",
          json.dumps(
            measurement.values,
            indent=2,
          ),
        ))
        registers = loader.measurement_to_registers(measurement)
        server.set_registers(registers)

    server_task = asyncio.create_task(server.run())
    update_task = asyncio.create_task(update())

    await asyncio.gather(
      server_task,
      update_task,
    )

  if args.is_client():
    client = Client(
      ip_address=args.address(),
      slave_id=args.slave_id(),
      port=args.port(),
    )
    while True:
      await execute_client_requests(
        client,
        device_type,
        requests,
      )


async def execute_client_requests(
  client: Client,
  device_type: DeviceType,
  requests: List[Union[ReadRequest, WriteRequest]],
):
  log.info(("Executing requests for", device_type))
  start = time.time()
  for request in requests:
    if isinstance(request, ReadRequest):
      (registers, value) = await client.read(
        register=request.register,
        count=request.size,
        convert=request.convert,
      )
      log.info((
        "Read",
        request.name,
        request.register,
        registers,
        value,
      ))
    else:
      await client.write(register=request.register, values=request.values)
      log.info((
        "Wrote",
        request.name,
        request.register,
        request.values,
      ))
  end = time.time()
  log.info(("Took", end - start, "\n"))


def main():
  args = Args()

  async def wrapper(args: Args):
    try:
      await async_main(args)
    except asyncio.CancelledError:
      pass

  def exception_handler(
    __loop__: asyncio.AbstractEventLoop,
    __ctx__: dict[str, Any],
  ):
    pass

  loop = asyncio.get_event_loop()
  loop.set_exception_handler(exception_handler)

  main_task = asyncio.ensure_future(wrapper(args))
  for signal in [SIGINT, SIGTERM]:
    loop.add_signal_handler(signal, main_task.cancel)

  try:
    loop.run_until_complete(main_task)
  finally:
    loop.close()


if __name__ == "__main__":
  main()
