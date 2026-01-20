use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::{HeaderValue, Request, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, TextEncoder,
    register_histogram_vec, register_int_counter, register_int_counter_vec,
};
use std::sync::OnceLock;
use std::time::Instant;

const METRIC_PREFIX: &str = "gateway_control";

/// Metrics 注册结果，可能包含注册错误
type MetricRegistration<T> = Result<T, prometheus::Error>;

fn http_requests_total() -> &'static MetricRegistration<IntCounterVec> {
    static METRIC: OnceLock<MetricRegistration<IntCounterVec>> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_http_requests_total"),
                "控制平面 HTTP 请求总数"
            ),
            &["method", "path", "status"]
        )
    })
}

fn http_request_duration_seconds() -> &'static MetricRegistration<HistogramVec> {
    static METRIC: OnceLock<MetricRegistration<HistogramVec>> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = HistogramOpts::new(
            format!("{METRIC_PREFIX}_http_request_duration_seconds"),
            "控制平面 HTTP 请求耗时（秒）",
        );
        register_histogram_vec!(opts, &["method", "path"])
    })
}

fn audit_write_failures_total() -> &'static MetricRegistration<IntCounter> {
    static METRIC: OnceLock<MetricRegistration<IntCounter>> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter!(
            format!("{METRIC_PREFIX}_audit_write_failures_total"),
            "控制平面审计写入失败次数"
        )
    })
}

pub fn inc_audit_write_failure() {
    if let Ok(counter) = audit_write_failures_total() {
        counter.inc();
    }
}

pub async fn metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().as_str().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "<unmatched>".to_string());

    let start = Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();

    let status = response.status().as_u16().to_string();

    if let Ok(counter) = http_requests_total() {
        counter.with_label_values(&[&method, &path, &status]).inc();
    }
    if let Ok(histogram) = http_request_duration_seconds() {
        histogram
            .with_label_values(&[&method, &path])
            .observe(elapsed);
    }

    response
}

pub fn render_metrics() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            let mut resp = Response::new(Body::from(buffer));
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(encoder.format_type())
                    .unwrap_or_else(|_| HeaderValue::from_static("text/plain; version=0.0.4")),
            );
            resp
        }
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            // 使用已知的有效值构造响应，避免 panic
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Metrics encoding failed",
            )
                .into_response()
        }
    }
}
