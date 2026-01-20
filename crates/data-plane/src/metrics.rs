use prometheus::{
    HistogramOpts, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Opts,
    register_histogram_vec, register_int_counter_vec, register_int_gauge, register_int_gauge_vec,
};
use std::sync::OnceLock;

const METRIC_PREFIX: &str = "gateway_data";

fn requests_total() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_http_requests_total"),
                "数据平面 HTTP 请求总数"
            ),
            &["method", "status"]
        )
        .expect("register metric")
    })
}

fn request_duration_seconds() -> &'static HistogramVec {
    static METRIC: OnceLock<HistogramVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        let opts = HistogramOpts::new(
            format!("{METRIC_PREFIX}_http_request_duration_seconds"),
            "数据平面 HTTP 请求耗时（秒）",
        );
        register_histogram_vec!(opts, &["method", "status"]).expect("register metric")
    })
}

fn inflight_requests() -> &'static IntGauge {
    static METRIC: OnceLock<IntGauge> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_gauge!(
            format!("{METRIC_PREFIX}_inflight_requests"),
            "数据平面当前进行中的请求数（包含 WS 存量连接）"
        )
        .expect("register metric")
    })
}

fn upstream_errors_total() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_counter_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_upstream_errors_total"),
                "数据平面与上游交互错误总数"
            ),
            &["reason"]
        )
        .expect("register metric")
    })
}

fn upstream_target_healthy() -> &'static IntGaugeVec {
    static METRIC: OnceLock<IntGaugeVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        register_int_gauge_vec!(
            Opts::new(
                format!("{METRIC_PREFIX}_upstream_target_healthy"),
                "上游目标健康状态（1=健康，0=不健康）"
            ),
            &["pool_id", "address"]
        )
        .expect("register metric")
    })
}

pub fn observe_request(method: &str, status: u16, seconds: f64) {
    let status = status.to_string();
    requests_total().with_label_values(&[method, &status]).inc();
    request_duration_seconds()
        .with_label_values(&[method, &status])
        .observe(seconds);
}

pub fn inflight_inc() {
    inflight_requests().inc();
}

pub fn inflight_dec() {
    inflight_requests().dec();
}

pub fn inc_upstream_error(reason: &str) {
    upstream_errors_total().with_label_values(&[reason]).inc();
}

pub fn set_target_health(pool_id: &str, address: &str, healthy: bool) {
    upstream_target_healthy()
        .with_label_values(&[pool_id, address])
        .set(if healthy { 1 } else { 0 });
}
