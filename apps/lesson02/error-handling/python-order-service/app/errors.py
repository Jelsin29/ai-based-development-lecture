"""Uygulamanın hata modeli.

Python'da hata yönetiminin birincil aracı exception'dır. Rust'taki enum'a benzer bir
düzen kurmak için ortak bir taban sınıftan türeyen bir hiyerarşi tanımlıyoruz. Böylece:

- Her hata türü kendi HTTP durum kodunu ve metrik etiketini (kind) taşır,
- Tek bir handler (`AppError`) tüm beklenen hataları aynı formatta cevaplar,
- `AppError` dışında kalan her şey "beklenmeyen hata" yani bug kabul edilir.
"""

from __future__ import annotations

import logging


class AppError(Exception):
    """Beklenen (iş kuralı / dış bağımlılık kaynaklı) tüm hataların atası."""

    status_code: int = 500
    kind: str = "internal"
    log_level: int = logging.ERROR

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.message = message


class NotFoundError(AppError):
    status_code = 404
    kind = "not_found"
    log_level = logging.WARNING

    def __init__(self, order_id: int) -> None:
        super().__init__(f"{order_id} numaralı sipariş bulunamadı")
        self.order_id = order_id


class ValidationError(AppError):
    status_code = 422
    kind = "validation"
    log_level = logging.WARNING


class OutOfStockError(AppError):
    status_code = 409
    kind = "out_of_stock"
    log_level = logging.WARNING

    def __init__(self, product: str, requested: int, available: int) -> None:
        super().__init__(
            f"'{product}' için yetersiz stok (istenen: {requested}, mevcut: {available})"
        )
        self.product, self.requested, self.available = product, requested, available


class PaymentGatewayError(AppError):
    """Dış ödeme servisinin tüm denemelere rağmen başarısız olması."""

    status_code = 503
    kind = "payment_gateway"
    log_level = logging.ERROR

    def __init__(self) -> None:
        super().__init__("ödeme işlemi tamamlanamadı")


# --- Dış servis seviyesindeki (düşük seviye) hatalar ---------------------------------
# Bunlar AppError DEĞİLDİR; katman sınırında `raise PaymentGatewayError() from exc`
# ile uygulama hatasına çevrilir. `from` sayesinde orijinal sebep (__cause__) kaybolmaz.


class GatewayTimeout(Exception):
    def __init__(self, ms: int) -> None:
        super().__init__(f"ödeme sağlayıcısı {ms} ms içinde cevap vermedi")


class GatewayRejected(Exception):
    def __init__(self, status: int) -> None:
        super().__init__(f"ödeme sağlayıcısı isteği {status} durum koduyla reddetti")
