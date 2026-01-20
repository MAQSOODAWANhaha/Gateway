use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::{HeaderValue, Request, header};
use axum::middleware::Next;
use axum::response::Response;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, TextEncoder,
    register_histogram_vec, register_int_counter, register_int_counter_vec,
};
use std::sync::OnceLock;
use std::time::Instant;

const METRIC_PREFIX: &str = "gateway_control";

fn http_requests_total() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_http_requests_total"),
                "控制平面 HTTP 请求总数"
            ),
            &["method", "path", "status"]
        )
        .expect("register metric")
    })
}

fn http_request_duration_seconds() -> &'static HistogramVec {
    static METRIC: OnceLock<HistogramVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = HistogramOpts::new(
            format!("{METRIC_PREFIX}_http_request_duration_seconds"),
            "控制平面 HTTP 请求耗时（秒）",
        );
        register_histogram_vec!(opts, &["method", "path"]).expect("register metric")
    })
}

fn audit_write_failures_total() -> &'static IntCounter {
    static METRIC: OnceLock<IntCounter> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter!(
            format!("{METRIC_PREFIX}_audit_write_failures_total"),
            "控制平面审计写入失败次数"
        )
        .expect("register metric")
    })
}

pub fn inc_audit_write_failure() {
    audit_write_failures_total().inc();
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
    http_requests_total()
        .with_label_values(&[&method, &path, &status])
        .inc();
    http_request_duration_seconds()
        .with_label_values(&[&method, &path])
        .observe(elapsed);

    response
}

pub fn render_metrics() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("encode metrics");

    let mut resp = Response::new(Body::from(buffer));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(encoder.format_type())
            .unwrap_or_else(|_| HeaderValue::from_static("text/plain; version=0.0.4")),
    );
    resp
}
