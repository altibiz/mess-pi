from typing import Any, NamedTuple, Callable, Union
from probe.client import Client

# TODO: from config


class ReadRequest(NamedTuple):
  name: str
  register: int
  size: int
  convert: Callable[..., Any]


class WriteRequest(NamedTuple):
  name: str
  register: int
  values: list[int]


abb_b2x_requests: list[Union[ReadRequest, WriteRequest]] = [
  # abb-B2x
  ReadRequest(
    name="Detect",
    register=0x8960,
    size=6,
    convert=Client.to_ascii,
  ),
  ReadRequest(
    name="Id",
    register=0x8900,
    size=2,
    convert=Client.to_uint32,
  ),
  # WriteRequest(
  #   name="Configuration", register=0x8C90, values=[0x01]),
  # ReadRequest(
  #   name="Configuration",
  #   register=0x8C90,
  #   size=1,
  #   convert=Client.to_hex,
  # ),
  # WriteRequest(name="Tariff daily", register=0x8A07, values=[0x01]),
  # WriteRequest(name="Tariff nightly", register=0x8A07, values=[0x02]),
  # ReadRequest(
  #   name="Tariff",
  #   register=0x8A07,
  #   size=1,
  #   convert=Client.to_hex,
  # ),
  # abb-B2x instantaneous
  ReadRequest(
    name="Voltage L1",
    register=0x5B00,
    size=2,
    convert=Client.to_uint32,
  ),
  ReadRequest(
    name="Voltage L2",
    register=0x5B02,
    size=2,
    convert=Client.to_uint32,
  ),
  ReadRequest(
    name="Voltage L3",
    register=0x5B04,
    size=2,
    convert=Client.to_uint32,
  ),
  ReadRequest(
    name="Current L1",
    register=0x5B0C,
    size=2,
    convert=Client.to_uint32,
  ),
  ReadRequest(
    name="Current L2",
    register=0x5B0E,
    size=2,
    convert=Client.to_uint32,
  ),
  ReadRequest(
    name="Current L3",
    register=0x5B10,
    size=2,
    convert=Client.to_uint32,
  ),
  ReadRequest(
    name="Active Power L1",
    register=0x5B16,
    size=2,
    convert=Client.to_sint32,
  ),
  ReadRequest(
    name="Active Power L2",
    register=0x5B18,
    size=2,
    convert=Client.to_sint32,
  ),
  ReadRequest(
    name="Active Power L3",
    register=0x5B1A,
    size=2,
    convert=Client.to_sint32,
  ),
  ReadRequest(
    name="Reactive Power L1",
    register=0x5B1E,
    size=2,
    convert=Client.to_sint32,
  ),
  ReadRequest(
    name="Reactive Power L2",
    register=0x5B20,
    size=2,
    convert=Client.to_sint32,
  ),
  ReadRequest(
    name="Reactive Power L3",
    register=0x5B22,
    size=2,
    convert=Client.to_sint32,
  ),
  # abb-B2x cumulative
  ReadRequest(
    name="Active Energy Import L1",
    register=0x5460,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import L2",
    register=0x5464,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import L3",
    register=0x5468,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Export L1",
    register=0x546C,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Export L2",
    register=0x5470,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Export L3",
    register=0x5474,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Import L1",
    register=0x5484,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Import L2",
    register=0x5488,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Import L3",
    register=0x548C,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Export L1",
    register=0x5490,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Export L2",
    register=0x5494,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Export L3",
    register=0x5498,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import Total",
    register=0x5000,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Export Total",
    register=0x5004,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Import Total",
    register=0x500C,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Export Total",
    register=0x5010,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import Total T1",
    register=0x5170,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import Total T2",
    register=0x5174,
    size=4,
    convert=Client.to_uint64,
  ),
]

schneider_iem3xxx_requests: list[Union[ReadRequest, WriteRequest]] = [
  # schneider-iEM3xxx
  ReadRequest(
    name="Detect",
    register=0x0031,
    size=20,
    convert=Client.to_utf8,
  ),
  ReadRequest(
    name="Id",
    register=0x0081,
    size=2,
    convert=Client.to_uint32,
  ),
  # WriteRequest(
  #   name="Configuration", register=5249, values=[2060, 0x0000, 0x0001]),
  # WriteRequest(
  #   name="Tariff daily", register=5249, values=[2008, 0x0000, 0x0001]),
  # WriteRequest(name="Tariff nightly",
  #              register=5249,
  #              values=[2008, 0x0000, 0x0002]),
  # ReadRequest(
  #   name="Tariff",
  #   register=5374,
  #   size=2,
  #   convert=Client.to_registers,
  # ),
  # schneider-iEM3xxx instantaneous
  ReadRequest(
    name="Voltage L1",
    register=0x0BD3,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Voltage L2",
    register=0x0BD5,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Voltage L3",
    register=0x0BD7,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Current L1",
    register=0x0BB7,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Current L2",
    register=0x0BB9,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Current L3",
    register=0x0BBB,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Active Power L1",
    register=0x0BED,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Active Power L2",
    register=0x0BEF,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Active Power L3",
    register=0x0BF1,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Reactive Power Total",
    register=0x0BFB,
    size=2,
    convert=Client.to_float32,
  ),
  ReadRequest(
    name="Apparent Power Total",
    register=0x0C03,
    size=2,
    convert=Client.to_float32,
  ),
  # schneider-iEM3xxx cumulative
  ReadRequest(
    name="Active Energy Import L1",
    register=0x0DBD,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import L2",
    register=0x0DC1,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import L3",
    register=0x0DC5,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import Total",
    register=0x0C83,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Export Total",
    register=0x0C87,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Import Total",
    register=0x0C93,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Reactive Energy Export Total",
    register=0x0C97,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import Total T1",
    register=0x1063,
    size=4,
    convert=Client.to_uint64,
  ),
  ReadRequest(
    name="Active Energy Import Total T2",
    register=0x1067,
    size=4,
    convert=Client.to_uint64,
  ),
]
