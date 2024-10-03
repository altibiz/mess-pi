import struct
import asyncio
from typing import Callable, Optional, TypeVar, Union, List
from pymodbus.client import AsyncModbusTcpClient

TRead = TypeVar("TRead")


class Client:

  def __init__(
    self,
    ip_address: str,
    slave_id: int,
    port: int,
  ):
    self.__ip_address = ip_address
    self.__slave_id = slave_id
    self.__port = port
    self.__modbus_connected = False
    self.__modbus_client = self.__create_client()

  def __del__(self):
    self.__modbus_client.close()

  def __reopen(self):
    self.__modbus_connected = False
    self.__modbus_client.close()
    self.__modbus_client = self.__create_client()

  def __create_client(self):
    return AsyncModbusTcpClient(
      host=self.__ip_address,
      port=self.__port,
      retries=0,
      timeout=0.1,
      retry_on_empty=False,
    )

  async def read(
    self,
    register: int,
    count: int,
    convert: Callable[..., TRead],
  ) -> tuple[list[int], TRead]:
    while True:
      if not self.__modbus_connected:
        try:
          await self.__modbus_client.connect()
          self.__modbus_connected = True
        except Exception:
          await asyncio.sleep(1)
          continue

      registers: Optional[List[int]] = None
      try:
        response = await asyncio.wait_for(
          self.__modbus_client.read_holding_registers(
            address=register,
            count=count,
            slave=self.__slave_id,
          ),
          timeout=1)
        if response.isError():
          continue

        registers = response.registers  # pyright: ignore unknownMemberType

      except Exception:
        await asyncio.sleep(1)
        self.__reopen()
        continue

      try:
        value = convert(*registers)
        return (registers, value)
      except Exception:
        await asyncio.sleep(1)
        continue

  async def write(
    self,
    register: int,
    values: list[int],
  ) -> None:
    while True:
      if not self.__modbus_connected:
        try:
          await self.__modbus_client.connect()
          self.__modbus_connected = True
        except Exception:
          await asyncio.sleep(1)
          continue

      try:
        response = await asyncio.wait_for(self.__modbus_client.write_registers(
          address=register,
          values=values,
          slave=self.__slave_id,
        ),
                                          timeout=1)
        if response.isError():
          await asyncio.sleep(1)
          break

      except Exception:
        await asyncio.sleep(1)
        self.__reopen()
        continue

      break

  @staticmethod
  def multiplied_by(converter: Callable[..., Union[int, float]],
                    multiplier: float) -> Callable[..., float]:

    def result(*registers: int) -> float:
      converted = converter(*registers)
      multiplied = converted * multiplier
      return multiplied

    return result

  @staticmethod
  def to_uint32(first: int, second: int) -> int:
    return (first << 16) | second

  @staticmethod
  def to_uint64(first: int, second: int, third: int, fourth: int) -> int:
    return (first << 48) | (second << 32) | (third << 16) | fourth

  @staticmethod
  def to_sint32(upper_half: int, lower_half: int) -> int:
    combined_value = (upper_half << 16) | lower_half

    if combined_value & 0x80000000:
      combined_value -= 0x100000000

    return combined_value

  @staticmethod
  def to_sint64(first: int, second: int, third: int, fourth: int) -> int:
    combined_value = (first << 48) | (second << 32) | (third << 16) | fourth

    if combined_value & 0x8000000000000000:
      combined_value -= 0x10000000000000000

    return combined_value

  @staticmethod
  def to_sint16(register: int) -> int:
    return register if register < 0x8000 else register - 0x10000

  @staticmethod
  def to_float32(upper_half: int, lower_half: int) -> float:
    upper_bytes = struct.pack("!H", upper_half)
    lower_bytes = struct.pack("!H", lower_half)
    return struct.unpack("!f", upper_bytes + lower_bytes)[0]

  @staticmethod
  def to_raw_bytes(*registers: int) -> List[int]:
    return [
      uint8 for uint16 in registers
      for uint8 in [(uint16 >> 8) & 0xFF, uint16 & 0xFF]
    ]

  @staticmethod
  def to_bytes(*registers: int) -> bytes:
    return bytes(Client.to_raw_bytes(*registers))

  @staticmethod
  def to_hex(*registers: int) -> list[str]:
    return [f"{register:04x}" for register in registers]

  @staticmethod
  def to_registers(*registers: int) -> list[int]:
    return [register for register in registers]

  @staticmethod
  def to_ascii(*registers: int) -> str:
    return Client.to_bytes(*registers).decode("ascii")

  @staticmethod
  def to_utf8(*registers: int) -> str:
    return Client.to_bytes(*registers).decode("utf8")
