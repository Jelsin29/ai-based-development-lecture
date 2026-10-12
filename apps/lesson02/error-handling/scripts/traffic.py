"""Sipariş servislerine karışık (başarılı + hatalı) trafik üretir.

Sadece Python standart kütüphanesini kullanır; ek paket gerektirmez.

    python scripts/traffic.py                 # iki servise birden, 60 saniye
    python scripts/traffic.py --duration 300 --bug-rate 0.05
    python scripts/traffic.py --targets http://localhost:8081
"""

from __future__ import annotations

import argparse
import json
import random
import time
import urllib.error
import urllib.request
from collections import Counter

PRODUCTS = ["klavye", "mouse", "monitor", "kulaklik"]  # "kulaklik" stokta yok -> 409


def call(method: str, url: str, body: dict | None = None) -> int:
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return resp.status
    except urllib.error.HTTPError as e:
        return e.code
    except (urllib.error.URLError, TimeoutError):
        return 0  # servis ayakta değil


def one_round(base: str, bug_rate: float) -> list[int]:
    statuses = []
    r = random.random()
    if r < bug_rate:
        statuses.append(call("GET", f"{base}/debug/bug"))
    elif r < 0.15:
        statuses.append(call("GET", f"{base}/orders/{random.randint(1000, 9999)}"))  # 404
    elif r < 0.25:
        statuses.append(call("POST", f"{base}/orders", {"product": "mouse", "quantity": 0}))  # 422
    else:
        product = random.choice(PRODUCTS)
        status = call("POST", f"{base}/orders", {"product": product, "quantity": 1})
        statuses.append(status)
        if status == 201:
            # Son oluşturulan siparişi bilmediğimiz için rastgele küçük bir id ile ödeme dene
            statuses.append(call("POST", f"{base}/orders/{random.randint(1, 20)}/pay"))
    return statuses


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--targets", nargs="+", default=["http://localhost:8081", "http://localhost:8082"])
    p.add_argument("--duration", type=int, default=60, help="saniye")
    p.add_argument("--rps", type=float, default=5, help="servis başına saniyedeki tur sayısı")
    p.add_argument("--bug-rate", type=float, default=0.02, help="/debug/bug çağrılma olasılığı")
    args = p.parse_args()

    stats: Counter[str] = Counter()
    end = time.time() + args.duration
    while time.time() < end:
        for base in args.targets:
            for status in one_round(base, args.bug_rate):
                stats[str(status)] += 1
        time.sleep(1 / args.rps)
        print("\r" + "  ".join(f"{k}:{v}" for k, v in sorted(stats.items())), end="", flush=True)
    print("\nbitti.")


if __name__ == "__main__":
    main()
