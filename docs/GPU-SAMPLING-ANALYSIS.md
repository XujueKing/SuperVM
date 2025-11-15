# GPU Sampling Analysis: Single vs Multi-Sample Robustness

Generated at: 2025-11-12 16:24:45

Single-run file: data/gpu_threshold_scan/windows_dx12_20251112_154810.csv
Multi-run file (5 samples): data/gpu_threshold_scan/windows_dx12_5samples_20251112.csv

## Statistical Comparison

| Size | Device | Single (ms) | Multi Median (ms) | Multi P90 (ms) | Multi Min-Max | Variance Reduction |
|------|--------|-------------|-------------------|----------------|---------------|-------------------|
| 100000 | Cpu | 2 | 2 | 2 | 2-2 | +100% |
| 100000 | Gpu | 2 | 2 | 2 | 2-2 | +100% |
| 1000000 | Cpu | 22 | 24 | 25 | 22-25 | +86.4% |
| 1000000 | Gpu | 6 | 6 | 6 | 5-6 | +83.3% |
| 20000 | Cpu | 0 | 0 | 0 | 0-0 | N/A |
| 20000 | Gpu | 17 | 1 | 20 | 1-20 | -11.8% |
| 250000 | Cpu | 6 | 6 | 9 | 6-9 | +50% |
| 250000 | Gpu | 3 | 2 | 3 | 2-3 | +66.7% |
| 50000 | Cpu | 1 | 1 | 1 | 1-1 | +100% |
| 50000 | Gpu | 6 | 2 | 2 | 2-2 | +100% |
| 500000 | Cpu | 11 | 12 | 12 | 12-12 | +100% |
| 500000 | Gpu | 4 | 3 | 5 | 3-5 | +50% |

## Key Observations

**Sampling Benefits**:

- Multi-sample runs provide median and P90 percentiles, reducing noise

- Min-Max range shows measurement variability

- Variance reduction indicates stability improvement over single runs

**When to Use Multi-Sampling**:

- Final production thresholds (use 5-10 samples for P90 values)

- Performance regression detection

- Unstable or noisy environments

**When Single-Run is Sufficient**:

- Quick development iteration

- Relative comparisons (e.g., DX12 vs GLES trend)

- Stable test environment with low variance

## Recommendations

1. **Development Phase**: Use single-run scans for fast iteration
2. **Validation Phase**: Use 5-sample runs to confirm thresholds
3. **Production Deployment**: Use 10+ samples for P95/P99 confidence
4. **CI/CD**: Run single-sample for trend detection, multi-sample weekly

