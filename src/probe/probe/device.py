from __future__ import annotations
from enum import Enum
from typing import Optional


class DeviceType(Enum):
  abb_b2x = 'abb-B2x'
  schneider_iem3xxx = 'schneider-iEM3xxx'

  def __str__(self):
    return self.value

  @staticmethod
  def from_string(string: str) -> Optional[DeviceType]:
    for member in DeviceType:
      if member.value == string:
        return member

    return None
