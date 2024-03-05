import asyncio
import time
from typing import Any, List, NamedTuple, Callable, Union
from client import Client
from args import Args
from device import DeviceType


class ReadRequest(NamedTuple):
  name: str
  register: int
  size: int
  convert: Callable[..., Any]


class WriteRequest(NamedTuple):
  name: str
  register: int
  values: list[int]


async def main():
  args = Args()

  client = Client(
    ip_address=args.ip_address(),
    slave_id=args.slave_id(),
  )

  if args.device_type() == DeviceType.abb:
    while True:
      await execute(client, DeviceType.abb, [
        ReadRequest(
          name="Type designation",
          register=0x8960,
          size=6,
          convert=Client.to_ascii,
        ),
        ReadRequest(
          name="Serial number",
          register=0x8900,
          size=2,
          convert=Client.to_uint32,
        ),
        ReadRequest(
          name="Active power",
          register=0x5B14,
          size=2,
          convert=Client.to_sint32,
        ),
        ReadRequest(
          name="Active power export L1",
          register=0x546C,
          size=4,
          convert=Client.to_raw_bytes,
        ),
        ReadRequest(
          name="Reactive Power",
          register=0x5B1C,
          size=2,
          convert=Client.to_raw_bytes,
        ),
        ReadRequest(
          name="Reactive Import",
          register=0x500C,
          size=2,
          convert=Client.to_uint32,
        ),
        ReadRequest(
          name="Reactive Export",
          register=0x5010,
          size=2,
          convert=Client.to_uint32,
        ),
        ReadRequest(
          name="Reactive Net",
          register=0x5014,
          size=2,
          convert=Client.to_sint32,
        ),
        ReadRequest(
          name="Active Import",
          register=0x5000,
          size=2,
          convert=Client.to_uint32,
        ),
        ReadRequest(
          name="Active Export",
          register=0x5004,
          size=2,
          convert=Client.to_uint32,
        ),
        ReadRequest(
          name="Active Net",
          register=0x5008,
          size=2,
          convert=Client.to_sint32,
        ),
        ReadRequest(
          name="Tariff configuration",
          register=0x8C90,
          size=1,
          convert=Client.to_raw_bytes,
        ),
        ReadRequest(
          name="Tariff",
          register=0x8A07,
          size=1,
          convert=Client.to_raw_bytes,
        ),
      ])

  if args.device_type() == DeviceType.schneider:
    while True:
      await execute(client, DeviceType.schneider, [
        ReadRequest(
          name="Model",
          register=0x0031,
          size=20,
          convert=Client.to_utf8,
        ),
        ReadRequest(
          name="Serial number",
          register=0x0081,
          size=2,
          convert=Client.to_uint32,
        ),
        ReadRequest(
          name="Active Power",
          register=0x0BF3,
          size=2,
          convert=Client.to_float32,
        ),
        ReadRequest(
          name="Active energy import total",
          register=0x0C83,
          size=4,
          convert=Client.to_sint64,
        ),
        ReadRequest(
          name="Active energy import L1",
          register=0x0DBD,
          size=4,
          convert=Client.to_sint64,
        ),
        ReadRequest(
          name="Active energy import L2",
          register=0x0DC1,
          size=4,
          convert=Client.to_sint64,
        ),
        ReadRequest(
          name="Active energy import L3",
          register=0x0DC5,
          size=4,
          convert=Client.to_sint64,
        ),
        WriteRequest(name="Tariff daily", register=0x105E, values=[0x0001]),
        ReadRequest(
          name="Tariff",
          register=0x105E,
          size=1,
          convert=Client.to_raw_bytes,
        ),
        WriteRequest(name="Tariff nightly", register=0x105E, values=[0x0002]),
        ReadRequest(
          name="Tariff",
          register=0x105E,
          size=1,
          convert=Client.to_raw_bytes,
        ),
      ])


async def execute(client: Client, device_type: DeviceType,
                  requests: List[Union[ReadRequest, WriteRequest]]):
  print("Reading", device_type)
  start = time.time()
  for request in requests:
    if isinstance(request, ReadRequest):
      value = await client.read(
        register=request.register,
        count=request.size,
        convert=request.convert,
      )
      print("Read", request.name, value)
    else:
      await client.write(register=request.register, values=request.values)
      print("Wrote", request.name)
  end = time.time()
  print("took", end - start, "\n")


if __name__ == "__main__":
  asyncio.run(main())
