import logging
import os
from datetime import datetime, timezone
from typing import Any


class __TupleLogger(logging.Logger):

  def _log(  # type: ignore
      self,
      level: int,
      msg: str,
      *args: Any,
      **kwargs: Any,
  ):
    if isinstance(msg, tuple):
      msg = ' '.join(map(str, msg))
    super()._log(level, msg, *args, **kwargs)  # type: ignore


class __UtcFormatter(logging.Formatter):

  def formatTime(
    self,
    record: logging.LogRecord,
    datefmt: str | None = None,
  ):
    created = datetime.fromtimestamp(record.created, timezone.utc)
    if datefmt:
      result = created.strftime(datefmt)
    else:
      try:
        result = created.isoformat(timespec='microseconds') + "Z"
      except TypeError:
        result = created.isoformat() + "Z"
    return result


__stream_handler = logging.StreamHandler()
__stream_handler.setFormatter(
  __UtcFormatter("%(asctime)s %(levelname)+8s %(name)s: %(message)s"))
logging.basicConfig(
  level=logging.WARNING,
  handlers=[__stream_handler],
)
logging.setLoggerClass(__TupleLogger)
log = logging.getLogger("probe")

if os.getenv("PIDGEON_PROBE_ENV") == "development":
  log.setLevel(logging.DEBUG)
elif os.getenv("PIDGEON_PROBE_ENV") == "production":
  log.setLevel(logging.INFO)
else:
  log.setLevel(logging.DEBUG)
