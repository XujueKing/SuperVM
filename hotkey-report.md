# Hot Key LFU Report (ownership_sharding_mixed_bench)

鍙傛暟: decay_period=10 decay_factor=0.9 batches=1 hot_key_thr=5 adaptive=False tx_per_thread=200 threads=8 batch_size=20

| Medium | High | ExtremeTx | MediumTx | BatchTx | TPS | Conflicts | HotKeyThr | AdaptiveConf | Duration(ms) |
|--------|------|----------|---------|---------|-----|-----------|-----------|-------------|-------------|
| 2040 | 50120 | 0 | 0 | 12 | 303622 | 1307.3 / run | Avg Conflicts | 3 | 1.000 | NA |
