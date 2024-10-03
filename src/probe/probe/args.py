import re
from typing import Callable
from argparse import ArgumentParser
from probe.device import DeviceType
from probe.func import compose


class Args:

  def __init__(self):
    self.__parser = ArgumentParser(prog="probe", description="Pidgeon probe")
    self.__subparsers = self.__parser.add_subparsers(
      dest='command',
      required=True,
    )

    self.__client_parser = self.__subparsers.add_parser(
      'client',
      help='Client mode',
    )
    self.__client_parser.add_argument(
      "-a",
      "--address",
      required=False,
      default="127.0.0.1",
      type=Args.__regex(r"^((25[0-5]|(2[0-4]|1[0-9]|[1-9]|)[0-9])\.?\b){4}$"),
      help="IP address of the modbus gateway",
    )
    self.__client_parser.add_argument(
      "-p",
      "--port",
      required=False,
      default=502,
      type=int,
      help="Port of the modbus gateway",
    )
    self.__client_parser.add_argument(
      "-s",
      "--slave-id",
      required=False,
      default=0,
      type=compose(Args.__regex("^25[0-5]|(2[0-4]|1[0-9]|[1-9]|)[0-9]$"), int),
      help="Slave ID of the modbus device",
    )
    self.__client_parser.add_argument(
      "-d",
      "--device-type",
      required=True,
      type=DeviceType.from_string,
      help="Type of the modbus device",
      choices=list(DeviceType),
    )
    self.__client_parser.add_argument(
      "-c",
      "--config",
      required=False,
      type=str,
      help="Path to the config",
    )

    self.__server_parser = self.__subparsers.add_parser(
      'server',
      help='Server mode',
    )
    self.__server_parser.add_argument(
      "-a",
      "--address",
      required=False,
      default="0.0.0.0",
      type=Args.__regex(r"^((25[0-5]|(2[0-4]|1[0-9]|[1-9]|)[0-9])\.?\b){4}$"),
      help="IP address of the modbus server to bind to",
    )
    self.__server_parser.add_argument(
      "-p",
      "--port",
      required=False,
      default=502,
      type=int,
      help="Port of the modbus server to bind to",
    )
    self.__server_parser.add_argument(
      "-d",
      "--device-type",
      required=True,
      type=DeviceType.from_string,
      help="Type of the modbus device",
      choices=list(DeviceType),
    )
    self.__server_parser.add_argument(
      "-i",
      "--id",
      required=True,
      type=str,
      help="Id of modbus device",
    )
    self.__server_parser.add_argument(
      "-c",
      "--config",
      required=False,
      type=str,
      help="Path to the config",
    )
    self.__server_parser.add_argument(
      "-m",
      "--measurements",
      required=False,
      type=str,
      help="Path to the measurement folder",
    )

    self.__args = self.__parser.parse_args()

  def is_server(self) -> bool:
    return self.__args.command == "server"

  def is_client(self) -> bool:
    return self.__args.command == "client"

  def address(self) -> str:
    return self.__args.address

  def port(self) -> int:
    return self.__args.port

  def slave_id(self) -> int:
    return self.__args.slave_id

  def device_type(self) -> DeviceType:
    return self.__args.device_type

  def id(self) -> str:
    return self.__args.id

  def config(self) -> str | None:
    return self.__args.config

  def measurements(self) -> str | None:
    return self.__args.measurements

  @staticmethod
  def __regex(regex: str) -> Callable[[str], str]:
    compiled = re.compile(regex)

    def coerce(string: str) -> str:
      string = str(string)

      if not compiled.match(string):
        raise ValueError(f"string '{string}' did not match regex '{regex}'")

      return string

    return coerce
