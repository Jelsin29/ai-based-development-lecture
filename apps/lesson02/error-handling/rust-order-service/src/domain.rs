//! Basit sipariş alanı: bellek içi depo ve dış ödeme servisi simülasyonu.

use std::{collections::HashMap, sync::Mutex, time::Duration};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, GatewayError};

#[derive(Debug, Clone, Serialize)]
pub struct Order {
    pub id: u32,
    pub product: String,
    pub quantity: u32,
    pub paid: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewOrder {
    pub product: String,
    pub quantity: u32,
}

impl NewOrder {
    /// Doğrulama hatası bir istisna değil, sıradan bir dönüş değeridir.
    pub fn validate(&self) -> Result<(), AppError> {
        if self.product.trim().is_empty() {
            return Err(AppError::Validation("ürün adı boş olamaz".into()));
        }
        if !(1..=100).contains(&self.quantity) {
            return Err(AppError::Validation(format!(
                "miktar 1 ile 100 arasında olmalı, gelen: {}",
                self.quantity
            )));
        }
        Ok(())
    }
}

#[derive(Default)]
struct Inner {
    next_id: u32,
    orders: HashMap<u32, Order>,
    stock: HashMap<String, u32>,
}

pub struct OrderStore {
    inner: Mutex<Inner>,
}

impl OrderStore {
    pub fn with_sample_data() -> Self {
        let mut inner = Inner {
            next_id: 1,
            ..Default::default()
        };
        inner.stock.insert("klavye".into(), 50);
        inner.stock.insert("mouse".into(), 100);
        inner.stock.insert("monitor".into(), 5);
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn get(&self, id: u32) -> Result<Order, AppError> {
        // `lock()` yalnızca başka bir thread kilidi tutarken panic olduysa (poisoned) hata döner.
        // Bu bir programlama hatasıdır ve kurtarılamaz kabul edilir; bu yüzden `expect` uygundur.
        let inner = self.inner.lock().expect("order store mutex poisoned");
        inner.orders.get(&id).cloned().ok_or(AppError::NotFound(id))
    }

    pub fn create(&self, req: NewOrder) -> Result<Order, AppError> {
        req.validate()?; // Hata varsa fonksiyondan erken döner (early return).

        let mut inner = self.inner.lock().expect("order store mutex poisoned");
        let available = inner.stock.get(&req.product).copied().unwrap_or(0);
        if available < req.quantity {
            return Err(AppError::OutOfStock {
                product: req.product,
                requested: req.quantity,
                available,
            });
        }
        inner
            .stock
            .insert(req.product.clone(), available - req.quantity);

        let id = inner.next_id;
        inner.next_id += 1;
        let order = Order {
            id,
            product: req.product,
            quantity: req.quantity,
            paid: false,
        };
        inner.orders.insert(id, order.clone());
        Ok(order)
    }

    pub fn mark_paid(&self, id: u32) -> Result<Order, AppError> {
        let mut inner = self.inner.lock().expect("order store mutex poisoned");
        let order = inner.orders.get_mut(&id).ok_or(AppError::NotFound(id))?;
        order.paid = true;
        Ok(order.clone())
    }
}

/// Güvenilmez bir dış ödeme servisini taklit eder: çağrıların bir kısmı zaman aşımına uğrar
/// veya reddedilir. `PAYMENT_FAILURE_RATE` ortam değişkeni ile hata oranı ayarlanabilir.
pub async fn call_payment_gateway(order_id: u32, failure_rate: f64) -> Result<(), GatewayError> {
    tokio::time::sleep(Duration::from_millis(fastrand::u64(20..120))).await;
    let roll = fastrand::f64();
    if roll < failure_rate / 2.0 {
        Err(GatewayError::Timeout(2000))
    } else if roll < failure_rate {
        Err(GatewayError::Rejected(502))
    } else {
        tracing::debug!(order_id, "ödeme sağlayıcısı onay verdi");
        Ok(())
    }
}

/// Geçici (transient) hatalar için üstel geri çekilmeli (exponential backoff) yeniden deneme.
pub async fn pay_with_retry(order_id: u32, failure_rate: f64) -> Result<(), GatewayError> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 1;
    loop {
        match call_payment_gateway(order_id, failure_rate).await {
            Ok(()) => return Ok(()),
            Err(e) if attempt < MAX_ATTEMPTS => {
                tracing::warn!(order_id, attempt, error = %e, "ödeme denemesi başarısız, tekrar denenecek");
                tokio::time::sleep(Duration::from_millis(50 * 2u64.pow(attempt))).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sifir_miktar_dogrulama_hatasi_doner() {
        let req = NewOrder {
            product: "mouse".into(),
            quantity: 0,
        };
        assert!(matches!(req.validate(), Err(AppError::Validation(_))));
    }

    #[test]
    fn olmayan_siparis_not_found_doner() {
        let store = OrderStore::with_sample_data();
        assert!(matches!(store.get(42), Err(AppError::NotFound(42))));
    }

    #[test]
    fn stok_yetersizse_out_of_stock_doner() {
        let store = OrderStore::with_sample_data();
        let res = store.create(NewOrder {
            product: "monitor".into(),
            quantity: 10,
        });
        assert!(matches!(
            res,
            Err(AppError::OutOfStock { available: 5, .. })
        ));
    }

    #[tokio::test]
    async fn hata_orani_sifirsa_odeme_basarili() {
        assert!(pay_with_retry(1, 0.0).await.is_ok());
    }

    #[tokio::test]
    async fn hata_orani_birse_odeme_basarisiz() {
        assert!(pay_with_retry(1, 1.0).await.is_err());
    }
}
