# GPU Backends Cross-Platform Summary

Generated at: 2025-11-12 16:13:06

Windows: data/gpu_threshold_scan/windows_compare_20251112.csv
Linux:   data/gpu_threshold_scan/linux_compare_20251112.csv

## Windows

| size | GLES (ms) | DX12 (ms) | winner |
|------|-----------|-----------|--------|
| 20000 | 17 | 17 | tie |
| 50000 | 3 | 6 | gles |
| 100000 | 4 | 2 | dx12 |
| 250000 | 4 | 3 | dx12 |
| 500000 | 9 | 4 | dx12 |
| 1000000 | 17 | 6 | dx12 |

## Linux (Vulkan)

| size | GLES (ms) | Vulkan (ms) | winner |
|------|-----------|-------------|--------|
| 20000 | 18 | 18 | vulkan |
| 50000 | 3 | 3 | vulkan |
| 100000 | 2 | 2 | vulkan |
| 250000 | 3 | 3 | vulkan |
| 500000 | 8 | 8 | vulkan |
| 1000000 | 16 | 16 | vulkan |

