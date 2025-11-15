# GPU Backend Comparison Summary (Windows)

Generated at: 2025-11-12 15:49:01
Input file: d:/WEB3_AI����/���������/data/gpu_threshold_scan/windows_compare_20251112.csv

| size | GLES GPU(ms) | DX12 GPU(ms) | winner |
|------|--------------|-------------|------|
| 20000 | 17 | 17 | tie |
| 50000 | 3 | 6 | gles |
| 100000 | 4 | 2 | dx12 |
| 250000 | 4 | 3 | dx12 |
| 500000 | 9 | 4 | dx12 |
| 1000000 | 17 | 6 | dx12 |

Summary: DX12 wins 4 times, GLES wins 1 times, ties 1.
First DX12-win size: 100000

Suggested threshold: use GPU(DX12) when size >= 100000; use CPU below.

CPU↔GPU thresholds (90% rule):

- DX12: GPU beats CPU at size >= 250000

- GLES: GPU beats CPU at size >= 250000
