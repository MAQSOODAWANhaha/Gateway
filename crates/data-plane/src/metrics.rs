use prometheus::{
    HistogramOpts, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Opts,
    register_histogram_vec, register_int_counter_vec, register_int_gauge, register_int_gauge_vec,
};
use std::sync::OnceLock;

const METRIC_PREFIX: &str = "gateway_data";

/// Metrics 注册结果，可能包含注册错误
type MetricRegistration<T> = Result<T, prometheus::Error>;

fn requests_total() -> &'static MetricRegistration<IntCounterVec> {
    static METRIC: OnceLock<MetricRegistration<IntCounterVec>> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_http_requests_total"),
                "数据平面 HTTP 请求总数"
            ),
            &["method", "status"]
        )
    })
}

fn request_duration_seconds() -> &'static MetricRegistration<HistogramVec> {
    static METRIC: OnceLock<MetricRegistration<HistogramVec>> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = HistogramOpts::new(
            format!("{METRIC_PREFIX}_http_request_duration_seconds"),
            "数据平面 HTTP 请求耗时（秒）",
        );
        register_histogram_vec!(opts, &["method", "status"])
    })
}

fn inflight_requests() -> &'static MetricRegistration<IntGauge> {
    static METRIC: OnceLock<MetricRegistration<IntGauge>> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_gauge!(
            format!("{METRIC_PREFIX}_inflight_requests"),
            "数据平面当前进行中的请求数（包含 WS 存量连接）"
        )
    })
}

fn upstream_errors_total() -> &'static MetricRegistration<IntCounterVec> {
    static METRIC: OnceLock<MetricRegistration<IntCounterVec>> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_upstream_errors_total"),
                "数据平面与上游交互错误总数"
            ),
            &["reason"]
        )
    })
}

fn upstream_target_healthy() -> &'static MetricRegistration<IntGaugeVec> {
    static METRIC: OnceLock<MetricRegistration<IntGaugeVec>> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_gauge_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_upstream_target_healthy"),
                "上游目标健康状态（1=健康，0=不健康）"
            ),
            &["pool_id", "address"]
        )
    })
}

pub fn observe_request(method: &str, status: u16, seconds: f64) {
    let status = status.to_string();
    if let Ok(counter) = requests_total() {
        counter.with_label_values(&[method, &status]).inc();
    }
    if let Ok(histogram) = request_duration_seconds() {
        histogram
            .with_label_values(&[method, &status])
            .observe(seconds);
    }
}

pub fn inflight_inc() {
    if let Ok(gauge) = inflight_requests() {
        gauge.inc();
    }
}

pub fn inflight_dec() {
    if let Ok(gauge) = inflight_requests() {
        gauge.dec();
    }
}

pub fn inc_upstream_error(reason: &str) {
    if let Ok(counter) = upstream_errors_total() {
        counter.with_label_values(&[reason]).inc();
    }
}

pub fn set_target_health(pool_id: &str, address: &str, healthy: bool) {
    if let Ok(gauge) = upstream_target_healthy() {
        gauge
            .with_label_values(&[pool_id, address])
            .set(if healthy { 1 } else { 0 });
    }
}
