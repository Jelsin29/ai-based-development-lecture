"""Basit sipariş alanı: bellek içi depo ve dış ödeme servisi simülasyonu."""

from __future__ import annotations

import asyncio
import logging
import random
import threading
from dataclasses import dataclass

from .errors import (
    GatewayRejected,
    GatewayTimeout,
    NotFoundError,
    OutOfStockError,
    PaymentGatewayError,
)

logger = logging.getLogger("orders.domain")


@dataclass
class Order:
    id: int
    product: str
    quantity: int
    paid: bool = False


class OrderStore:
    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._next_id = 1
        self._orders: dict[int, Order] = {}
        self._stock: dict[str, int] = {"klavye": 50, "mouse": 100, "monitor": 5}

    def get(self, order_id: int) -> Order:
        # EAFP (Easier to Ask Forgiveness than Permission): önce dene, olmazsa yakala.
        # Python'da idiomatik olan yaklaşım budur. Ancak KeyError'ı dışarı sızdırmıyoruz;
        # alan (domain) diline ait bir hataya çeviriyoruz.
        try:
            return self._orders[order_id]
        except KeyError:
            raise NotFoundError(order_id) from None  # 'from None': iç detay zincire eklenmez

    def create(self, product: str, quantity: int) -> Order:
        with self._lock:
            # LBYL (Look Before You Leap): önce kontrol et, sonra işlem yap.
            available = self._stock.get(product, 0)
            if available < quantity:
                raise OutOfStockError(product, quantity, available)
            self._stock[product] = available - quantity
            order = Order(id=self._next_id, product=product, quantity=quantity)
            self._orders[order.id] = order
            self._next_id += 1
            return order

    def mark_paid(self, order_id: int) -> Order:
        with self._lock:
            order = self.get(order_id)
            order.paid = True
            return order


async def call_payment_gateway(order_id: int, failure_rate: float) -> None:
    """Güvenilmez bir dış ödeme servisini taklit eder."""
    await asyncio.sleep(random.uniform(0.02, 0.12))
    roll = random.random()
    if roll < failure_rate / 2:
        raise GatewayTimeout(2000)
    if roll < failure_rate:
        raise GatewayRejected(502)
    logger.debug("ödeme sağlayıcısı onay verdi", extra={"order_id": order_id})


async def pay_with_retry(order_id: int, failure_rate: float, max_attempts: int = 3) -> None:
    """Geçici (transient) hatalar için üstel geri çekilmeli yeniden deneme."""
    for attempt in range(1, max_attempts + 1):
        try:
            await call_payment_gateway(order_id, failure_rate)
            return
        except (GatewayTimeout, GatewayRejected) as exc:
            if attempt == max_attempts:
                # Düşük seviyeli hatayı uygulama hatasına çevir, asıl sebebi zincirde koru.
                raise PaymentGatewayError() from exc
            logger.warning(
                "ödeme denemesi başarısız, tekrar denenecek",
                extra={"order_id": order_id, "attempt": attempt, "error": str(exc)},
            )
            await asyncio.sleep(0.05 * 2**attempt)
