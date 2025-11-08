// 单元测试 - Metrics 指标收集功能
// 测试 MetricsCollector 和 LatencyHistogram

#[cfg(test)]
mod metrics_tests {
    use crate::metrics::{LatencyHistogram, MetricsCollector};
    use crate::mvcc::{MvccStore, Txn};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_metrics_collector_basic() {
        let metrics = MetricsCollector::new();

        // 初始值应该为 0
        assert_eq!(
            metrics
                .txn_started
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            metrics
                .txn_committed
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            metrics
                .txn_aborted
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_metrics_tps_calculation() {
        let metrics = MetricsCollector::new();

        // 模拟一些事务
        for _ in 0..100 {
            metrics
                .txn_started
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            metrics
                .txn_committed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // TPS 应该 > 0
        let tps = metrics.tps();
        assert!(tps > 0.0, "TPS 应该大于 0");
    }

    #[test]
    fn test_metrics_success_rate() {
        let metrics = MetricsCollector::new();

        // 80 成功, 20 失败
        metrics
            .txn_started
            .store(100, std::sync::atomic::Ordering::Relaxed);
        metrics
            .txn_committed
            .store(80, std::sync::atomic::Ordering::Relaxed);
        metrics
            .txn_aborted
            .store(20, std::sync::atomic::Ordering::Relaxed);

        let success_rate = metrics.success_rate();
        assert!((success_rate - 80.0).abs() < 0.01, "成功率应该约为 80%");
    }

    #[test]
    fn test_latency_histogram_observe() {
        let histogram = LatencyHistogram::new();

        // 记录一些延迟
        histogram.observe(Duration::from_millis(1));
        histogram.observe(Duration::from_millis(5));
        histogram.observe(Duration::from_millis(10));
        histogram.observe(Duration::from_millis(100));

        // 验证延迟统计
        let (p50, p90, p99) = histogram.percentiles();
        assert!(p50 > 0.0);
        assert!(p90 >= p50);
        assert!(p99 >= p90);
    }

    #[test]
    fn test_latency_percentiles() {
        let histogram = LatencyHistogram::new();

        // 记录 100 个延迟值
        for i in 0..100 {
            histogram.observe(Duration::from_millis(i));
        }

        let (p50, p90, p99) = histogram.percentiles();

        // P50 应该在中间范围
        assert!(p50 >= 10.0 && p50 <= 100.0);
        // P90 应该大于 P50
        assert!(p90 > p50);
        // P99 应该大于 P90
        assert!(p99 >= p90);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = MetricsCollector::new();

        // 设置一些指标
        metrics
            .txn_started
            .store(1000, std::sync::atomic::Ordering::Relaxed);
        metrics
            .txn_committed
            .store(950, std::sync::atomic::Ordering::Relaxed);
        metrics
            .txn_aborted
            .store(50, std::sync::atomic::Ordering::Relaxed);

        // 导出 Prometheus 格式
        let output = metrics.export_prometheus();

        // 验证包含关键指标
        assert!(output.contains("mvcc_tps"));
        assert!(output.contains("mvcc_success_rate"));
        assert!(output.contains("mvcc_txn_started_total"));
        assert!(output.contains("mvcc_txn_committed_total"));
        assert!(output.contains("mvcc_txn_aborted_total"));
    }
}
