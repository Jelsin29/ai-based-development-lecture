mod domain;
mod error;
mod telemetry;

use std::{any::Any, sync::Arc, time::Instant};

use axum::{
    Json, Router,
    extract::{MatchedPath, Path, Request, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use tower_http::catch_panic::CatchPanicLayer;
use tracing::Instrument;

use domain::{NewOrder, Order, OrderStore};
use error::AppError;

struct AppState {
    store: OrderStore,
    payment_failure_rate: f64,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    telemetry::init_logging();

    let payment_failure_rate = std::env::var("PAYMENT_FAILURE_RATE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.3);

    let state = Arc::new(AppState {
        store: OrderStore::with_sample_data(),
        payment_failure_rate,
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/metrics", get(|| async { telemetry::render_metrics() }))
        .route("/orders", post(create_order))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/pay", post(pay_order))
        .route("/debug/bug", get(trigger_panic))
        .with_state(state)
        // Katman sırası önemlidir: observe en dışta olduğu için panic sonrası üretilen 500 de ölçülür.
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer(middleware::from_fn(observe));

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("{addr} adresi dinlenemedi: {e}"));

    tracing::info!(%addr, production = telemetry::is_production(), "rust-order-service başladı");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("sunucu beklenmedik şekilde sonlandı");
}

// ---------------------------------------------------------------- handlers

async fn get_order(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
) -> Result<Json<Order>, AppError> {
    let order = state.store.get(id)?;
    Ok(Json(order))
}

async fn create_order(
    State(state): State<SharedState>,
    payload: Result<Json<NewOrder>, JsonRejection>,
) -> Result<(StatusCode, Json<Order>), AppError> {
    let Json(req) = payload?; // JsonRejection -> AppError::Validation
    let order = state.store.create(req)?;
    tracing::info!(order_id = order.id, product = %order.product, "sipariş oluşturuldu");
    Ok((StatusCode::CREATED, Json(order)))
}

async fn pay_order(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
) -> Result<Json<Order>, AppError> {
    state.store.get(id)?; // önce sipariş var mı?
    // GatewayError -> AppError dönüşümü `#[from]` sayesinde `?` ile otomatik yapılır.
    domain::pay_with_retry(id, state.payment_failure_rate).await?;
    let order = state.store.mark_paid(id)?;
    tracing::info!(order_id = id, "ödeme alındı");
    Ok(Json(order))
}

/// Bilerek hatalı yazılmış kod: Kampanya kodunu sayıya çevirirken `unwrap()` kullanıyoruz.
/// "YUZDE10" gibi bir değer geldiğinde parse başarısız olur ve thread panic eder.
async fn trigger_panic() -> Json<serde_json::Value> {
    let campaign_code = "YUZDE10";
    let discount: u32 = campaign_code.parse().unwrap(); // <-- BUG
    Json(json!({ "discount": discount }))
}

// ---------------------------------------------------------------- middleware

/// Her istek için request_id üretir, bir tracing span'i açar, süre ve durum kodunu ölçer.
async fn observe(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    // Etiket olarak ham path (/orders/17) değil şablon (/orders/{id}) kullanılır;
    // aksi halde her id yeni bir zaman serisi üretir (high cardinality).
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".into());
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let span = tracing::info_span!("request", %request_id, %method, %route);
    let mut response = next.run(req).instrument(span.clone()).await;

    let status = response.status();
    let elapsed = start.elapsed();
    if route != "/metrics" {
        telemetry::HTTP_REQUESTS
            .with_label_values(&[method.as_str(), route.as_str(), status.as_str()])
            .inc();
        telemetry::HTTP_DURATION
            .with_label_values(&[route.as_str()])
            .observe(elapsed.as_secs_f64());
        span.in_scope(|| {
            tracing::info!(
                status = status.as_u16(),
                duration_ms = elapsed.as_millis() as u64,
                "istek tamamlandı"
            )
        });
    }

    if let Ok(v) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", v);
    }
    response
}

/// Panic'i 500 cevabına çevirir. Logu panic hook zaten yazdı; burada sayaçları artırıyoruz.
fn handle_panic(_err: Box<dyn Any + Send + 'static>) -> Response {
    telemetry::PANICS.inc();
    telemetry::APP_ERRORS.with_label_values(&["panic"]).inc();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": { "kind": "internal", "message": "Beklenmeyen bir hata oluştu." } })),
    )
        .into_response()
}
