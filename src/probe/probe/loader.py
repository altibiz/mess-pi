import csv
import tomllib
import os
import struct
import asyncio
from datetime import datetime, timezone
from dataclasses import dataclass
from typing import Any
from probe.log import log
from probe.device import DeviceType

# TODO: tariffs - mby not here
# TODO: meter id from command line
# TODO: make it less brittle
# remove dependency on consistent naming
# remove dependency on config structure


@dataclass
class Measurement:
  meter_id: str
  timestamp: datetime
  values: dict[str, str]


class Loader:

  METER_ID = "MeterId"
  TIMESTAMP = "Timestamp"
  CUMULATIVE = "Energy"

  __config: dict[str, Any]
  __measurements: list[Measurement]
  __device_type: DeviceType
  __id: str

  def __init__(
    self,
    device_type: DeviceType,
    id: str,
    config: str | None = None,
    measurements: str | None = None,
  ):
    script_dir = os.path.dirname(os.path.realpath(__file__))
    probe_dir = os.path.dirname(script_dir)
    root_dir = os.path.dirname(os.path.dirname(probe_dir))
    assets_dir = os.path.join(root_dir, "assets")

    self.__device_type = device_type
    self.__id = id

    if config is None:
      config = os.path.join(assets_dir, "config.toml")

    log.info(("Using config at", config))
    self.__config = Loader.__load_config(config)["modbus"]["devices"][str(
      device_type)]

    if measurements is not None:
      assets_dir = measurements

    if device_type == DeviceType.abb_b2x:
      measurements = os.path.join(
        assets_dir,
        "abb-B2x-measurements.csv",
      )
    elif device_type == DeviceType.schneider_iem3xxx:
      measurements = os.path.join(
        assets_dir,
        "schneider-iEM3xxx-measurements.csv",
      )
    else:
      raise ValueError("Device type unknown")

    log.info(("Using measurements at", measurements))
    self.__measurements = Loader.__load_measurements(measurements)

  def __aiter__(self) -> 'Loader':
    return self

  async def __anext__(self) -> Measurement:
    next_measurement = self.__next_measurement()
    await asyncio.sleep(
      (next_measurement.timestamp - datetime.now(timezone.utc)).total_seconds())
    return next_measurement

  def __iter__(self) -> 'Loader':
    return self

  def __next__(self) -> Measurement:
    return self.__next_measurement()

  def measurement_to_registers(
    self,
    measurement: Measurement,
  ) -> dict[int, int]:
    out: dict[int, int] = {}
    for config in (self.__config["measurement"] + self.__config["id"]):
      self.__write_measurement_or_id_register(measurement, config, out)
    self.__write_detect_register(out)
    return out

  def __next_measurement(self) -> Measurement:
    now = datetime.now(timezone.utc)
    first_measurement = self.__measurements[0]
    last_measurement = self.__measurements[-1]
    measurements_start = first_measurement.timestamp
    measurements_end = last_measurement.timestamp
    measurements_duration = measurements_end - measurements_start
    time_since_measurements_start = now - measurements_start
    measurements_start_delta = time_since_measurements_start % measurements_duration
    measurements_now = measurements_start + measurements_start_delta
    for measurement in self.__measurements:
      if measurement.timestamp >= measurements_now:
        values = measurement.values.copy()
        timestamp = now + (measurement.timestamp - measurements_now)
        time_since_measurements_start = timestamp - measurements_start
        cumulative_delta_factor = time_since_measurements_start / measurements_duration
        for cumulative in values.keys():
          if Loader.CUMULATIVE in cumulative:
            cumulative_start = float(first_measurement.values[cumulative])
            cumulative_end = float(last_measurement.values[cumulative])
            corrected = (
              cumulative_start +
              (cumulative_end - cumulative_start) * cumulative_delta_factor)
            last = float(values[cumulative])
            log.debug((
              "corrected",
              cumulative,
              ":",
              last,
              "->",
              corrected,
              "Δ",
              corrected - last,
            ))
            values[cumulative] = str(corrected)
        values[Loader.TIMESTAMP] = timestamp.isoformat()
        return Measurement(
          meter_id=measurement.meter_id,
          timestamp=timestamp,
          values=values,
        )
    raise OverflowError("Next measurement not found")

  def __write_measurement_or_id_register(
    self,
    measurement: Measurement,
    config: dict[str, Any],
    out: dict[int, int],
  ) -> None:

    def capitalize(x: str):
      return x[0].upper() + x[1:]

    name: None | str = None
    if "name" in config:
      name = capitalize(str(config["name"]))
    else:
      name = Loader.METER_ID

    address = int(config["address"])
    kind = str(next(iter(config["kind"])))
    multiplier = float(config["kind"][kind]["multiplier"])

    prev = measurement.values[name]
    if name == Loader.METER_ID:
      prev = self.__id

    value: None | int | float | str = None
    if kind == "string":
      value = str(prev)
    elif kind.startswith("f"):
      value = float(prev) / multiplier
    else:
      value = int(float(prev) / multiplier)

    data = Loader.__to_little_endian_bytes(value, kind)

    registers: list[int] | None
    if kind == "string":
      registers = Loader.__encode_string_bytes(data)
    else:
      registers = Loader.__encode_numeric_bytes(data)

    log.debug((
      "converted",
      measurement.values[name],
      "->",
      value,
      "->",
      data,
      "->",
      registers,
    ))

    for i, register in enumerate(registers):
      out[address + i] = register

  def __write_detect_register(self, out: dict[int, int]) -> None:
    start: int | None = None
    registers: list[int] | None = None

    if self.__device_type == DeviceType.abb_b2x:
      start = 35168
      # NOTE: B23 312-100
      registers = [16946, 13088, 13105, 12845, 12592, 12288]
    elif self.__device_type == DeviceType.schneider_iem3xxx:
      start = 49
      # NOTE: iEM3255
      registers = [
        26949, 19763, 12853, 13600, 8224, 8224, 8224, 8224, 8224, 8224, 8224,
        8224, 8224, 8224, 8224, 8224, 8224, 8224, 8224, 8224
      ]
    else:
      raise ValueError("Unsupported device type")

    for i, register in enumerate(registers):
      out[start + i] = register

  @staticmethod
  def __load_config(file_name: str) -> dict[str, Any]:
    with open(file_name, "rb") as config_file:
      return tomllib.load(config_file)

  @staticmethod
  def __load_measurements(file_name: str) -> list[Measurement]:
    with open(file_name) as file:
      reader = csv.DictReader(file)
      measurements: list[Measurement] = []
      for measurement in reader:
        parsed = Measurement(
          meter_id=measurement[Loader.METER_ID],
          timestamp=datetime.fromisoformat(measurement[Loader.TIMESTAMP]),
          values=measurement,
        )
        measurements.append(parsed)
      measurements.sort(key=lambda x: x.timestamp)
      return measurements

  @staticmethod
  def __encode_numeric_bytes(data: bytes) -> list[int]:
    data_list = list(data)
    if len(data) % 2 != 0:
      data_list.append(0)

    result: list[int] = []
    for i in range(0, len(data_list), 2):
      low_byte = data_list[i]
      high_byte = data_list[i + 1]
      value = (high_byte << 8) | low_byte
      result.append(value)

    result.reverse()

    return result

  @staticmethod
  def __encode_string_bytes(data: bytes) -> list[int]:
    data_list = list(data)
    if len(data) % 2 != 0:
      data_list.append(0)

    result: list[int] = []
    for i in range(0, len(data_list), 2):
      low_byte = data_list[i]
      high_byte = data_list[i + 1]
      value = (high_byte << 8) | low_byte
      result.append(value)

    result.reverse()

    return result

  @staticmethod
  def __to_little_endian_bytes(value: int | float | str, kind: str) -> bytes:
    if kind == "string" and isinstance(value, str):
      return value.encode("utf-8")
    elif kind == 'f32':
      return struct.pack('<f', value)
    elif kind == 'f64':
      return struct.pack('<d', value)
    elif kind == 'u8':
      return struct.pack('<B', value)
    elif kind == 'u16':
      return struct.pack('<H', value)
    elif kind == 'u32':
      return struct.pack('<I', value)
    elif kind == 'u64':
      return struct.pack('<Q', value)
    elif kind == 's8':
      return struct.pack('<b', value)
    elif kind == 's16':
      return struct.pack('<h', value)
    elif kind == 's32':
      return struct.pack('<i', value)
    elif kind == 's64':
      return struct.pack('<q', value)
    else:
      raise ValueError("Unsupported register kind")
