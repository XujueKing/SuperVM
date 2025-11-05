# GC 杩愯鏃跺彲瑙傛祴鎬?

閫傜敤鐗堟湰: v0.8.0+

鏈枃妗ｄ粙缁嶅浣曞湪杩愯鏃惰瀵?Auto GC 鐨勫叧閿弬鏁颁笌鐘舵€侊紝甯姪浣犲湪鍘嬫祴鎴栫敓浜т腑蹇€熻瘖鏂笌璋冧紭銆?

## 鑳藉姏姒傝

- 瀹炴椂鑾峰彇褰撳墠 GC 鍛ㄦ湡涓庨槇鍊硷紙鍖呭惈鑷€傚簲璋冩暣鍚庣殑鍊硷級
- 纭鏄惁鍚敤浜嗚嚜閫傚簲 GC
- 鎼厤 GC 缁熻锛堟鏁般€佹竻鐞嗙増鏈暟锛夎繘琛屾晥鏋滆瘎浼?

## API

閫氳繃 `MvccStore::get_auto_gc_runtime()` 鑾峰彇蹇収锛?

```rust
use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};

let store = MvccStore::new_with_config(GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,
        version_threshold: 1000,
        run_on_start: true,
        enable_adaptive: true,
    }),
});

if let Some(rt) = store.get_auto_gc_runtime() {
    println!(
        "adaptive: {}, interval_secs: {}, threshold: {}",
        rt.enable_adaptive, rt.interval_secs, rt.version_threshold
    );
}
```

杩斿洖缁撴瀯锛堝彧璇诲揩鐓э級锛?

```rust
pub struct AutoGcRuntime {
    pub enable_adaptive: bool,
    pub interval_secs: u64,       // 褰撳墠鐢熸晥鐨勯棿闅旓紙鍙兘琚嚜閫傚簲璋冩暣锛?
    pub version_threshold: u64,   // 褰撳墠鐢熸晥鐨勯槇鍊硷紙鍙兘琚嚜閫傚簲璋冩暣锛?
}
```

> 娉細褰撴湭閰嶇疆 `auto_gc` 鏃讹紝`get_auto_gc_runtime()` 杩斿洖 `None`銆?

## 涓?GC 缁熻鑱斿悎浣跨敤

```rust
let stats = store.get_gc_stats();
println!(
    "gc_count={}, versions_cleaned={}, keys_cleaned={}, last_gc_ts={}",
    stats.gc_count, stats.versions_cleaned, stats.keys_cleaned, stats.last_gc_ts
);
```

缁撳悎 `AutoGcRuntime` 鍙互鍒ゆ柇锛?
- 璋冩暣鍚庣殑闂撮殧/闃堝€兼槸鍚︿笌鏈熸湜涓€鑷?
- 鍦ㄦ煇涓尯闂村唴 GC 鏄惁鏈夋晥锛堟竻鐞嗙巼锛?

## 鍏稿瀷鐢ㄦ硶锛氬帇娴嬭娴嬬偣

鍦ㄥ帇娴嬪惊鐜垨闃舵鎬?sleep 澶勬彃鍏ヨ娴嬶細

```rust
if let Some(rt) = store.get_auto_gc_runtime() {
    let stats = store.get_gc_stats();
    println!(
        "[obs] interval={}s, threshold={}, gc_count={}, cleaned_versions={}",
        rt.interval_secs, rt.version_threshold, stats.gc_count, stats.versions_cleaned
    );
}
```

## 鎺掗敊寤鸿

- 鐗堟湰鏁板眳楂樹笉涓嬶細
  - 妫€鏌?`interval_secs` 鏄惁杩囧ぇ銆乣version_threshold` 鏄惁杩囬珮
  - 纭鏄惁鍚敤浜?`enable_adaptive`
- GC 娓呯悊鐜囪繃浣庯細
  - 閫傚綋闄嶄綆闃堝€兼垨缂╃煭闂撮殧
  - 妫€鏌ユ槸鍚﹀瓨鍦ㄩ暱浜嬪姟闃绘尅鍘嗗彶鐗堟湰鍥炴敹

## 鐩稿叧鏂囨。

- 鍘嬪姏娴嬭瘯涓庤皟浼樻寚鍗? ./stress-testing-guide.md
- 骞惰鎵ц璁捐: ./parallel-execution.md




