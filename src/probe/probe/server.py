import random
import time
from pymodbus.datastore import (
  ModbusServerContext,
  ModbusSlaveContext,
  ModbusSparseDataBlock,
)
from pymodbus.device import ModbusDeviceIdentification
from pymodbus.server import StartAsyncTcpServer  # pyright: ignore unknownMemberType


class Server:

  class ModbusDelayedSlaveContext(ModbusSlaveContext):

    def getValues(self, fc_as_hex, address, count=1):  # type: ignore
      delay = random.uniform(0.01, 0.05)
      time.sleep(delay)
      return super().getValues(fc_as_hex, address, count)  # type: ignore

  class ModbusZeroedSparseDataBlock(ModbusSparseDataBlock):

    def validate(self, address: int, count: int = 1):
      return bool([(address, count), True][1])

    def getValues(self, address: int, count: int = 1):
      out: list[int] = []
      for i in range(count):
        if (address + i) in self.values:
          out.append(self.values[address + i])
        else:
          out.append(0)
      return out

  def __init__(
    self,
    address: str,
    port: int,
    registers: dict[int, int],
  ):
    self.__address = address
    self.__port = port
    self.__context = self.__create_context(registers)
    self.__identification = self.__create_identification()

  def __create_context(self, registers: dict[int, int]):
    slave = Server.ModbusDelayedSlaveContext(
      hr=Server.ModbusZeroedSparseDataBlock(registers),
      zero_mode=True,
    )
    # NOTE: single makes it always return the slave for any slave number...
    # NOTE: using slave 1 because the tcp device slave number seems to not work
    return ModbusServerContext(slaves={1: slave}, single=False)

  def __create_identification(self):
    identity = ModbusDeviceIdentification()
    identity.VendorName = "Pymodbus"
    identity.ProductCode = "PM"
    identity.VendorUrl = "http://github.com/riptideio/pymodbus"
    identity.ProductName = "Pymodbus Server"
    identity.ModelName = "Pymodbus Server"
    identity.MajorMinorRevision = "3.5.0"

    return identity

  def set_registers(self, registers: dict[int, int]):
    self.__context[1].setValues(  # pyright: ignore unknownMemberType
      3,
      0,
      registers,
    )

  async def run(self):
    await StartAsyncTcpServer(
      context=self.__context,
      identity=self.__identification,
      address=(self.__address, self.__port),
    )
