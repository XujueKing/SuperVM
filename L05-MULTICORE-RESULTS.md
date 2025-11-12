# L0.5 Multi-Core Partitioned FastPath Benchmark Results
**Date**: 2025-11-12  
**Branch**: king/l0-mvcc-privacy-verification

## Test Configuration
- **Transaction Count**: 200,000 per run
- **Simulated Cycles**: 32 (per transaction workload)
- **Partition Counts**: 2, 4, 8
- **Feature**: `partitioned-fastpath`
- **Build**: `--release`

## Results Summary

| Partitions | TPS | Elapsed (ms) | Speedup vs 2-core | Efficiency (%) |
|------------|-----|--------------|-------------------|----------------|
| 2 (baseline) | 2,579,051 | 77.55 | 1.00x | 50.0% |
| 4 | 5,960,079 | 33.56 | 2.31x | 57.8% |
| 8 | 6,917,974 | 28.91 | 2.68x | 33.5% |

*Efficiency = (Speedup / Partitions) × 100%*

## Observations
- **Linear Scaling**: 2→4核获得2.31x加速，接近理论值；4→8核仅提升16%，存在瓶颈
- **Contention Points**: 8核效率降至33.5%，疑似全局Injector队列竞争激烈
- **Optimal Partition Count**: 4核（效率57.8%）为当前架构甜点；8核边际收益低
- **Peak Performance**: 6.92M TPS (8核) - 原型级表现，真实FastPath集成后预期10M+ TPS

## Next Steps
- [ ] Test with `thread-local-ts` feature enabled
- [ ] Test with `smallvec-chains` + `dashmap-mvcc` combo
- [ ] Benchmark real FastPath executor (not just simulated cycles)
- [ ] NUMA-aware thread affinity binding
- [ ] Update ROADMAP.md L0.5 progress to 100%

## Raw Data
See: `bench_partitioned_fastpath_results.csv`
