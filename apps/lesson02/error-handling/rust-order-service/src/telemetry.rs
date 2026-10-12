//! Loglama ve metrik altyapısı.
//!
//! - Geliştirme ortamında (APP_ENV=development) okunabilir, renkli loglar,
//! - Üretim ortamında (APP_ENV=production) makinece işlenebilir JSON loglar üretilir.
//!   Loglar her zaman stdout'a yazılır; toplanması altyapının (Alloy -> Loki) işidir.

use std::sync::LazyLock;

use prometheus::{
    HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry, TextEncoder,
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

pub static HTTP_REQUESTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    let c = IntCounterVec::new(
        Opts::new("http_requests_total", "Toplam HTTP istek sayısı"),
        &["method", "route", "status"],
    )
    .expect("metric tanımı geçerli olmalı");
    REGISTRY
        .register(Box::new(c.clone()))
        .expect("metric kaydı");
    c
});

pub static HTTP_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    let h = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP istek süresi"),
        &["route"],
    )
    .expect("metric tanımı geçerli olmalı");
    REGISTRY
        .register(Box::new(h.clone()))
        .expect("metric kaydı");
    h
});

pub static APP_ERRORS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    let c = IntCounterVec::new(
        Opts::new("app_errors_total", "Türüne göre uygulama hataları"),
        &["kind"],
    )
    .expect("metric tanımı geçerli olmalı");
    REGISTRY
        .register(Box::new(c.clone()))
        .expect("metric kaydı");
    c
});

pub static PANICS: LazyLock<IntCounter> = LazyLock::new(|| {
    let c = IntCounter::new("app_panics_total", "Yakalanan panic sayısı").expect("metric");
    REGISTRY
        .register(Box::new(c.clone()))
        .expect("metric kaydı");
    c
});

pub fn is_production() -> bool {
    std::env::var("APP_ENV").is_ok_and(|v| v == "production")
}

pub fn init_logging() {
    // RUST_LOG ile seviye ayarlanabilir: RUST_LOG=debug cargo run
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if is_production() {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_span_list(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init();
    }

    // Panic'ler normalde stderr'e düz metin olarak basılır. Hook ile onları da yapısal log'a çeviriyoruz.
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "bilinmeyen panic".into());
        // RUST_BACKTRACE=1 ise backtrace'i de logla (geliştirme ortamında çok faydalıdır).
        let backtrace = std::backtrace::Backtrace::capture();
        if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            tracing::error!(error.kind = "panic", panic.location = %location, panic.backtrace = %backtrace, "PANIC: {payload}");
        } else {
            tracing::error!(error.kind = "panic", panic.location = %location, "PANIC: {payload}");
        }
    }));
}

pub fn render_metrics() -> String {
    TextEncoder::new()
        .encode_to_string(&REGISTRY.gather())
        .unwrap_or_else(|e| format!("# metric encode hatası: {e}"))
}
