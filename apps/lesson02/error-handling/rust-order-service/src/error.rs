//! Uygulamanın hata modeli.
//!
//! Rust'ta exception yoktur. Kurtarılabilir (recoverable) hatalar `Result<T, E>` ile
//! değer olarak döner, kurtarılamaz (unrecoverable) durumlar ise `panic!` ile ifade edilir.
//! Burada tüm beklenen hata durumlarını tek bir enum içinde topluyoruz. Böylece derleyici,
//! bir hata türünü ele almayı unuttuğumuzda (non-exhaustive match) bizi uyarır.

use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::telemetry;

/// Dış servis (ödeme sağlayıcısı) kaynaklı hatalar.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("ödeme sağlayıcısı {0} ms içinde cevap vermedi")]
    Timeout(u64),
    #[error("ödeme sağlayıcısı isteği {0} durum koduyla reddetti")]
    Rejected(u16),
}

/// Uygulama seviyesindeki tüm beklenen hatalar.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0} numaralı sipariş bulunamadı")]
    NotFound(u32),

    #[error("geçersiz istek: {0}")]
    Validation(String),

    #[error("'{product}' için yetersiz stok (istenen: {requested}, mevcut: {available})")]
    OutOfStock {
        product: String,
        requested: u32,
        available: u32,
    },

    /// `#[from]` sayesinde `GatewayError` döndüren bir fonksiyonu `?` ile çağırdığımızda
    /// hata otomatik olarak `AppError::PaymentGateway`'e dönüştürülür.
    #[error("ödeme işlemi tamamlanamadı")]
    PaymentGateway(#[from] GatewayError),
}

/// Bozuk/eksik JSON gövdesi de bir doğrulama hatasıdır. axum'un kendi ret (rejection)
/// tipini kendi hata modelimize çeviriyoruz ki tüm hatalar aynı formatta dönsün.
impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        AppError::Validation(rejection.body_text())
    }
}

impl AppError {
    /// Metriklerde ve loglarda kullanılan, düşük kardinaliteli hata türü etiketi.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "not_found",
            AppError::Validation(_) => "validation",
            AppError::OutOfStock { .. } => "out_of_stock",
            AppError::PaymentGateway(_) => "payment_gateway",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::OutOfStock { .. } => StatusCode::CONFLICT,
            AppError::PaymentGateway(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

/// Hatanın HTTP cevabına dönüştüğü TEK nokta. Loglama ve metrik de burada yapılır;
/// böylece handler'lar sadece `?` kullanır, hata yönetimi kod tabanına dağılmaz.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.kind();
        telemetry::APP_ERRORS.with_label_values(&[kind]).inc();

        if status.is_server_error() {
            // Hata zincirinin (source) tamamını logla: "ödeme işlemi tamamlanamadı" -> "... cevap vermedi"
            let cause = std::error::Error::source(&self)
                .map(|s| s.to_string())
                .unwrap_or_default();
            tracing::error!(error.kind = kind, error.cause = %cause, "{self}");
        } else {
            // 4xx hatalar istemci kaynaklıdır; sistemin sağlığıyla ilgili değildir.
            tracing::warn!(error.kind = kind, "{self}");
        }

        // Üretim ortamında 5xx hatalarının iç detayını istemciye sızdırmıyoruz.
        let message = if status.is_server_error() && telemetry::is_production() {
            "İşlem şu anda gerçekleştirilemiyor, lütfen daha sonra tekrar deneyin.".to_string()
        } else {
            self.to_string()
        };

        (
            status,
            Json(json!({ "error": { "kind": kind, "message": message } })),
        )
            .into_response()
    }
}
