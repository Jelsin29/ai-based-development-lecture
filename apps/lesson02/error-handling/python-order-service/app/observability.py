"""Loglama ve metrik altyapısı.

- APP_ENV=development  -> okunabilir, tek satırlık loglar + tam traceback
- APP_ENV=production   -> her satırı bir JSON nesnesi olan yapısal (structured) loglar

Loglar her zaman stdout'a yazılır (12-Factor App, "Logs" ilkesi). Dosyaya yazmak,
döndürmek (rotate) veya merkezi bir sisteme taşımak uygulamanın değil altyapının işidir.
"""

from __future__ import annotations

import contextvars
import json
import logging
import os
import sys
from datetime import datetime, timezone

from prometheus_client import Counter, Histogram, disable_created_metrics

disable_created_metrics()  # *_created zaman serilerini üretme (gereksiz gürültü)

# Her isteğe ait request_id'yi, fonksiyonlara parametre geçirmeden loglara taşımak için.
request_id_var: contextvars.ContextVar[str] = contextvars.ContextVar("request_id", default="-")


def is_production() -> bool:
    return os.getenv("APP_ENV", "development") == "production"


# ------------------------------------------------------------------------- metrikler
HTTP_REQUESTS = Counter(
    "http_requests_total", "Toplam HTTP istek sayısı", ["method", "route", "status"]
)
HTTP_DURATION = Histogram("http_request_duration_seconds", "HTTP istek süresi", ["route"])
APP_ERRORS = Counter("app_errors_total", "Türüne göre uygulama hataları", ["kind"])
UNHANDLED = Counter("app_unhandled_exceptions_total", "Yakalanmamış exception sayısı")


# ------------------------------------------------------------------------- loglama
class RequestIdFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        record.request_id = request_id_var.get()
        return True


_STD_ATTRS = set(vars(logging.makeLogRecord({})).keys()) | {"message", "request_id"}


class JsonFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        payload = {
            "timestamp": datetime.fromtimestamp(record.created, tz=timezone.utc).isoformat(),
            "level": record.levelname,
            "logger": record.name,
            "message": record.getMessage(),
            "request_id": getattr(record, "request_id", "-"),
        }
        # logger.info("...", extra={"order_id": 5}) ile gelen alanları da ekle
        for key, value in vars(record).items():
            if key not in _STD_ATTRS:
                payload[key] = value
        if record.exc_info:
            exc = record.exc_info[1]
            payload["exception.type"] = type(exc).__name__ if exc else None
            payload["exception.stacktrace"] = self.formatException(record.exc_info)
        return json.dumps(payload, ensure_ascii=False, default=str)


def setup_logging() -> None:
    handler = logging.StreamHandler(sys.stdout)
    handler.addFilter(RequestIdFilter())
    if is_production():
        handler.setFormatter(JsonFormatter())
    else:
        handler.setFormatter(
            logging.Formatter("%(asctime)s %(levelname)-7s [%(request_id)s] %(name)s: %(message)s")
        )

    root = logging.getLogger()
    root.handlers[:] = [handler]
    root.setLevel(os.getenv("LOG_LEVEL", "INFO"))
    # uvicorn'un kendi access log'unu kapatıyoruz; istek logunu middleware'de biz yazıyoruz.
    logging.getLogger("uvicorn.access").disabled = True
