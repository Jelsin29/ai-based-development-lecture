"""Hata yollarını (unhappy path) test etmek, mutlu yolu test etmek kadar önemlidir."""

from fastapi.testclient import TestClient

from app import domain
from app.main import app

# raise_server_exceptions=False: yakalanmamış exception'ı teste fırlatmak yerine 500 cevabını görelim
client = TestClient(app, raise_server_exceptions=False)


async def _no_sleep(_seconds):
    return None


def test_olmayan_siparis_404_doner():
    r = client.get("/orders/9999")
    assert r.status_code == 404
    assert r.json()["error"]["kind"] == "not_found"


def test_gecersiz_miktar_422_doner():
    r = client.post("/orders", json={"product": "mouse", "quantity": 0})
    assert r.status_code == 422
    assert r.json()["error"]["kind"] == "validation"


def test_stok_yetersizse_409_doner():
    r = client.post("/orders", json={"product": "monitor", "quantity": 50})
    assert r.status_code == 409
    assert r.json()["error"]["kind"] == "out_of_stock"


def test_odeme_saglayicisi_hep_hata_verirse_503_doner(monkeypatch):
    async def always_fail(order_id, failure_rate):
        raise domain.GatewayTimeout(2000)

    monkeypatch.setattr(domain, "call_payment_gateway", always_fail)
    monkeypatch.setattr(domain.asyncio, "sleep", _no_sleep)
    order = client.post("/orders", json={"product": "mouse", "quantity": 1}).json()
    r = client.post(f"/orders/{order['id']}/pay")
    assert r.status_code == 503
    assert r.json()["error"]["kind"] == "payment_gateway"


def test_bug_500_doner_ve_request_id_header_eklenir():
    r = client.get("/debug/bug", headers={"x-request-id": "test-123"})
    assert r.status_code == 500
    assert r.headers["x-request-id"] == "test-123"
