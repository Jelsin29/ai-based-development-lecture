"""python-order-service: Rust örneğiyle aynı API'yi sunan FastAPI servisi.

Çalıştırma (geliştirme):  uvicorn app.main:app --reload --port 8082
Çalıştırma (üretim)    :  APP_ENV=production uvicorn app.main:app --host 0.0.0.0 --port 8082
"""

from __future__ import annotations

import logging
import os
import time
import uuid

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from prometheus_client import make_asgi_app
from pydantic import BaseModel, Field

from . import observability as obs
from .domain import Order, OrderStore, pay_with_retry
from .errors import AppError

obs.setup_logging()
logger = logging.getLogger("orders.api")

PAYMENT_FAILURE_RATE = float(os.getenv("PAYMENT_FAILURE_RATE", "0.3"))

app = FastAPI(title="python-order-service")
app.mount("/metrics", make_asgi_app())
store = OrderStore()


class NewOrder(BaseModel):
    product: str = Field(min_length=1)
    quantity: int = Field(ge=1, le=100)


def error_body(kind: str, message: str) -> dict:
    return {"error": {"kind": kind, "message": message}}


# ---------------------------------------------------------------- exception handlers
@app.exception_handler(AppError)
async def handle_app_error(request: Request, exc: AppError) -> JSONResponse:
    """Beklenen hataların HTTP cevabına dönüştüğü TEK nokta."""
    obs.APP_ERRORS.labels(kind=exc.kind).inc()
    cause = repr(exc.__cause__) if exc.__cause__ else None
    logger.log(exc.log_level, exc.message, extra={"error.kind": exc.kind, "error.cause": cause})

    message = exc.message
    if exc.status_code >= 500 and obs.is_production():
        message = "İşlem şu anda gerçekleştirilemiyor, lütfen daha sonra tekrar deneyin."
    return JSONResponse(error_body(exc.kind, message), status_code=exc.status_code)


@app.exception_handler(RequestValidationError)
async def handle_validation_error(request: Request, exc: RequestValidationError) -> JSONResponse:
    obs.APP_ERRORS.labels(kind="validation").inc()
    details = "; ".join(f"{'.'.join(map(str, e['loc']))}: {e['msg']}" for e in exc.errors())
    logger.warning("geçersiz istek: %s", details, extra={"error.kind": "validation"})
    return JSONResponse(error_body("validation", f"geçersiz istek: {details}"), status_code=422)


# ---------------------------------------------------------------- middleware
@app.middleware("http")
async def observe(request: Request, call_next):
    """request_id üretir, süreyi ölçer ve yakalanmamış exception'ları 500'e çevirir."""
    request_id = request.headers.get("x-request-id") or str(uuid.uuid4())
    token = obs.request_id_var.set(request_id)
    start = time.perf_counter()
    try:
        try:
            response = await call_next(request)
        except Exception:
            # Buraya düşen her şey bir BUG'dır: AppError olmadığı için kimse beklemiyordu.
            obs.UNHANDLED.inc()
            obs.APP_ERRORS.labels(kind="unhandled").inc()
            logger.exception("yakalanmamış exception", extra={"error.kind": "unhandled"})
            response = JSONResponse(
                error_body("internal", "Beklenmeyen bir hata oluştu."), status_code=500
            )

        # Etiket olarak ham path (/orders/17) değil şablon (/orders/{order_id}) kullanılır;
        # aksi halde her id yeni bir zaman serisi üretir (high cardinality).
        route = getattr(request.scope.get("route"), "path", "unmatched")
        elapsed = time.perf_counter() - start
        if not request.url.path.startswith("/metrics"):
            obs.HTTP_REQUESTS.labels(request.method, route, str(response.status_code)).inc()
            obs.HTTP_DURATION.labels(route).observe(elapsed)
            logger.info(
                "istek tamamlandı",
                extra={
                    "method": request.method,
                    "route": route,
                    "status": response.status_code,
                    "duration_ms": round(elapsed * 1000, 2),
                },
            )
        response.headers["x-request-id"] = request_id
        return response
    finally:
        obs.request_id_var.reset(token)


# ---------------------------------------------------------------- endpoints
@app.get("/health")
async def health() -> str:
    return "ok"


@app.get("/orders/{order_id}")
async def get_order(order_id: int) -> Order:
    return store.get(order_id)  # NotFoundError fırlatabilir, handler yakalar


@app.post("/orders", status_code=201)
async def create_order(req: NewOrder) -> Order:
    order = store.create(req.product, req.quantity)
    logger.info("sipariş oluşturuldu", extra={"order_id": order.id, "product": order.product})
    return order


@app.post("/orders/{order_id}/pay")
async def pay_order(order_id: int) -> Order:
    store.get(order_id)
    await pay_with_retry(order_id, PAYMENT_FAILURE_RATE)
    order = store.mark_paid(order_id)
    logger.info("ödeme alındı", extra={"order_id": order_id})
    return order


@app.get("/debug/bug")
async def trigger_bug() -> dict:
    """Bilerek hatalı yazılmış kod: kampanya kodunu sayıya çevirmeye çalışıyoruz."""
    campaign_code = "YUZDE10"
    discount = int(campaign_code)  # <-- BUG: ValueError fırlatır, kimse yakalamaz
    return {"discount": discount}
