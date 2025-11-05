# Groth16 zkSNARK 娣卞害瀛︿範绗旇
**Phase 2 Week 3 - Day 1**

## 姒傝

Groth16 鏄?2016 骞?Jens Groth 鎻愬嚭鐨?zkSNARK 璇佹槑绯荤粺锛岀浉姣?Pinocchio 鏈夋樉钁楁€ц兘鎻愬崌锛?
- **璇佹槑澶у皬**: 浠?3 涓兢鍏冪礌 (2涓狦1 + 1涓狦2) 鈮?128 bytes
- **楠岃瘉閫熷害**: 浠呴渶 3 娆￠厤瀵硅繍绠?(vs Pinocchio 鐨?12 娆?
- **搴旂敤**: Zcash Sapling, Filecoin, Loopring 绛?

---

## 鏍稿績姒傚康

### 1. R1CS (Rank-1 Constraint System)
灏嗚绠楄浆鎹负绾︽潫绯荤粺锛?

**绾︽潫鏍煎紡**: `A 路 w 鈯?B 路 w = C 路 w`
- `w`: 瑙佽瘉鍚戦噺 (witness vector) = `(1, w鈧? w鈧? ..., w鈧?`
- `A, B, C`: 绾︽潫鐭╅樀
- `鈯檂: Hadamard 绉?(閫愬厓绱犱箻娉?

**绀轰緥**: 璇佹槑 `x鲁 + x + 5 = 35`
```
鍙橀噺: w = (1, out, x, v鈧?
鍏朵腑:
  v鈧?= x * x      (涓棿鍙橀噺)
  out = v鈧?* x + x + 5

绾︽潫1 (v鈧?= x虏):
  A = [0, 0, 1, 0]  // x
  B = [0, 0, 1, 0]  // x  
  C = [0, 0, 0, 1]  // v鈧?
  => x 路 x = v鈧?

绾︽潫2 (out = v鈧伮穢 + x + 5):
  A = [0, 0, 1, 1]  // x + v鈧?
  B = [1, 0, 1, 0]  // 1 + x
  C = [5, 1, 0, 0]  // 5 + out
  => (x + v鈧? 路 (1 + x) = 5 + out
```

### 2. QAP (Quadratic Arithmetic Program)
灏?R1CS 杞崲涓哄椤瑰紡褰㈠紡锛?

**杞崲鍏紡**:
```
A路w 鈯?B路w = C路w  (n涓害鏉?
    鈫?
A(蟿) 路 B(蟿) = C(蟿) + H(蟿) 路 Z(蟿)

鍏朵腑:
- A(X) = 危 w岬?路 A岬?X)
- B(X) = 危 w岬?路 B岬?X)  
- C(X) = 危 w岬?路 C岬?X)
- Z(X) = (X-1)(X-2)(X-3)路路路(X-n)  (鐩爣澶氶」寮?
- H(X) = [A(X)路B(X) - C(X)] / Z(X)  (鍟嗗椤瑰紡)
```

**鍏抽敭鎬ц川**:
- 濡傛灉绾︽潫婊¤冻, 鍒?`A(X)路B(X) - C(X)` 蹇呰 `Z(X)` 鏁撮櫎
- 鍦ㄩ殢鏈虹偣 `蟿` 姹傚€奸獙璇?(Schwartz-Zippel 寮曠悊)

### 3. Trusted Setup (CRS)
鐢熸垚鍏叡鍙傝€冨瓧绗︿覆, 鍖呭惈"姣掓€у簾鏂? (toxic waste):

**绉樺瘑鍙傛暟**: `伪, 尾, 纬, 未, 蟿`
- **蹇呴』閿€姣?*: 浠讳綍浜虹煡閬撹繖浜涘€奸兘鑳戒吉閫犺瘉鏄?
- **涓€娆℃€?*: 姣忎釜鐢佃矾闇€瑕佸崟鐙?Setup

**Proving Key (璇佹槑瀵嗛挜)**:
- G1 鍏冪礌: 
  ```
  {伪, 未, 1, 蟿, 蟿虏, 蟿鲁, ..., 蟿鈦库伝鹿,
   L鈧椻倞鈧?蟿)/未, L鈧椻倞鈧?蟿)/未, ..., L鈧?蟿)/未,
   Z(蟿)/未, 蟿路Z(蟿)/未, 蟿虏路Z(蟿)/未, ..., 蟿鈦库伝虏路Z(蟿)/未}鈧?
  ```
- G2 鍏冪礌:
  ```
  {尾, 未, 1, 蟿, 蟿虏, 蟿鲁, ..., 蟿鈦库伝鹿}鈧?
  ```
- 鐢佃矾澶氶」寮忕郴鏁? `A鈧?X), A鈧?X), ..., B鈧?X), B鈧?X), ..., C鈧?X), C鈧?X), ...`

**Verification Key (楠岃瘉瀵嗛挜)**:
- G1 鍏冪礌: `{1, L鈧€(蟿)/纬, L鈧?蟿)/纬, ..., L鈧?蟿)/纬}鈧乣
- G2 鍏冪礌: `{1, 纬, 未}鈧俙
- Gt 鍏冪礌 (棰勮绠楅厤瀵?: `伪鈧?* 尾鈧俙

---

## Groth16 璇佹槑绯荤粺

### 璇佹槑鐢熸垚

**杈撳叆**:
- 瑙佽瘉鍚戦噺 `w = (1, w鈧? w鈧? ..., w鈧?`
- 闅忔満鏁?`r, s 鈭?饾斀鈧歚

**杈撳嚭**: 璇佹槑 `蟺 = (A, B, C)` (3涓兢鍏冪礌)

**璁＄畻鍏紡**:
```rust
// A 鈭?G1
A鈧?= 伪鈧?+ 危 w岬⒙稟岬?蟿)鈧?+ r路未鈧?

// B 鈭?G2  
B鈧?= 尾鈧?+ 危 w岬⒙稡岬?蟿)鈧?+ s路未鈧?

// C 鈭?G1
C鈧?= 危(w岬⒙稬岬?蟿)/未)鈧? (l+1 鍒?m)
   + H(蟿)路(Z(蟿)/未)鈧?
   + s路A鈧?+ r路B鈧?- r路s路未鈧?

鍏朵腑:
- L岬?X) = 尾路A岬?X) + 伪路B岬?X) + C岬?X)
- H(X) = [A(X)路B(X) - C(X)] / Z(X)
```

**姝ラ**:
1. 浠庤璇?`w` 璁＄畻澶氶」寮?`A(X), B(X), C(X)`
2. 璁＄畻鍟嗗椤瑰紡 `H(X) = [A(X)路B(X) - C(X)] / Z(X)`
3. 浣跨敤 CRS 涓殑鍔犲瘑鍊艰绠?`A, B, C`
4. 娣诲姞闅忔満鐩插寲鍥犲瓙 `r, s` (闆剁煡璇嗘€?

### 璇佹槑楠岃瘉

**杈撳叆**:
- 鍏紑杈撳叆 `w鈧€, w鈧? ..., w鈧梎
- 璇佹槑 `蟺 = (A, B, C)`

**楠岃瘉鏂圭▼**:
```
e(A, B) = e(伪, 尾) 路 e(危 w岬⒙稬岬?蟿)/纬, 纬) 路 e(C, 未)
              鈫?         鈫?                   鈫?
            棰勮绠?   鍏紑杈撳叆閮ㄥ垎          璇佹槑閮ㄥ垎
```

**浠呴渶 3 娆￠厤瀵?*:
- `e(A, B)` - 宸﹁竟
- `e(鍏紑杈撳叆, 纬)` - 鍙宠竟绗?椤?
- `e(C, 未)` - 鍙宠竟绗?椤?
- `e(伪, 尾)` - 棰勮绠? 涓嶈鍏?

**姝ｇ‘鎬ц瘉鏄?*:
```
宸﹁竟:
e(A, B) = e(伪 + A(蟿) + r路未, 尾 + B(蟿) + s路未)
        = A(蟿)路B(蟿) + 伪路尾 + 伪路B(蟿) + 尾路A(蟿) 
          + s路伪路未 + s路A(蟿)路未 + r路尾路未 + r路B(蟿)路未 + s路r路未虏

鍙宠竟:
e(伪,尾) 路 e(L(蟿)/纬, 纬) 路 e(C, 未)
= 伪路尾 + L(蟿) + [H(蟿)路Z(蟿) + s路A(蟿) + r路B(蟿) + r路s路未 - r路s路未]路未
= 伪路尾 + 尾路A(蟿) + 伪路B(蟿) + C(蟿) + H(蟿)路Z(蟿)
  + s路伪路未 + s路A(蟿)路未 + r路尾路未 + r路B(蟿)路未 + s路r路未虏

濡傛灉 A(蟿)路B(蟿) = C(蟿) + H(蟿)路Z(蟿), 鍒欎袱杈圭浉绛?鉁?
```

---

## 鍏抽敭鍒涙柊

### vs Pinocchio
1. **涓嶄娇鐢?"Knowledge of Coefficient"**
   - Pinocchio: 姣忎釜澶氶」寮忛渶瑕?2 涓兢鍏冪礌璇佹槑 "鐭ラ亾绯绘暟"
   - Groth16: 浣跨敤 伪, 尾 寮哄埗 A, B, C 浣跨敤鐩稿悓鐨?w

2. **鍒嗙鍏紑杈撳叆**
   - 浣跨敤 纬, 未 浣垮叕寮€杈撳叆鐙珛浜庣鏈夎璇?
   - 楠岃瘉鑰呭彧闇€澶勭悊鍏紑杈撳叆閮ㄥ垎

3. **璇佹槑鏇村皬**
   - Pinocchio: 7脳G1 + 1脳G2 鈮?256 bytes
   - Groth16: 2脳G1 + 1脳G2 鈮?128 bytes

---

## 閰嶅鍙嬪ソ鏇茬嚎

### BN254 (aka BN128)
```rust
// 鍩哄煙 饾斀鈧?(绱犳暟 p)
p = 21888242871839275222246405745257275088696311157297823662689037894645226208583

// 宓屽叆搴?k=12
// G1: E(饾斀鈧? - 鏅€氭き鍦嗘洸绾跨偣
// G2: E(饾斀鈧毬孤? - 鎵╁煙涓婄殑鐐?
// Gt: 饾斀鈧毬孤?- 閰嶅缁撴灉

// 鏇茬嚎鏂圭▼
E: y虏 = x鲁 + 3
```

**鎬ц兘**:
- G1 鐐瑰ぇ灏? 32 bytes (鍘嬬缉)
- G2 鐐瑰ぇ灏? 64 bytes (鍘嬬缉)
- 閰嶅杩愮畻: ~2-3ms (浼樺寲瀹炵幇)

### BLS12-381 (Zcash 鏂版爣鍑?
```rust
p = 4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787

// 鏇撮珮瀹夊叏鎬?(~128-bit vs BN254 鐨?~100-bit)
// 鏇撮€傚悎閫掑綊璇佹槑 (Halo2)
```

---

## 瀹為檯搴旂敤绀轰緥

### 鐢佃矾: 璇佹槑鐭ラ亾鍝堝笇鍘熷儚
```rust
// 鍏紑: hash = SHA256(secret)
// 绉佹湁: secret
// 鐢佃矾: 楠岃瘉 SHA256(secret) == hash

// R1CS 绾︽潫:
// SHA256 = ~25,000 涓害鏉?
// (姣忎釜甯冨皵杩愮畻/鍔犳硶/寮傛垨 鈫?绾︽潫)

绾︽潫鏁伴噺浼扮畻:
- SHA256: ~25K
- Pedersen Hash: ~5K  
- ECDSA 绛惧悕楠岃瘉: ~150K
- Merkle Tree (娣卞害20): ~20K
```

### 鎬ц兘鏁版嵁 (BN254 鏇茬嚎)
| 鐢佃矾澶嶆潅搴?| Setup 鏃堕棿 | 璇佹槑鏃堕棿 | 楠岃瘉鏃堕棿 | 璇佹槑澶у皬 |
|-----------|-----------|---------|---------|---------|
| 10K 绾︽潫 | ~5s | ~2s | ~5ms | 128 bytes |
| 100K 绾︽潫 | ~30s | ~10s | ~5ms | 128 bytes |
| 1M 绾︽潫 | ~5min | ~60s | ~5ms | 128 bytes |

**鍏抽敭**: 楠岃瘉鏃堕棿鍜岃瘉鏄庡ぇ灏?*涓嶉殢鐢佃矾瑙勬ā澧為暱** 鉁?

---

## SuperVM 搴旂敤鍦烘櫙

### 鍦烘櫙 1: 闅愯棌杞处閲戦
```rust
// 鍏紑: 
// - 杈撳叆鎵胯: C_in = v_in路H + r_in路G
// - 杈撳嚭鎵胯: C_out1, C_out2
// - 鎵嬬画璐? fee (鏄庢枃)

// 绉佹湁:
// - 杈撳叆閲戦: v_in
// - 杈撳嚭閲戦: v_out1, v_out2
// - 鐩插寲鍥犲瓙: r_in, r_out1, r_out2

// 鐢佃矾绾︽潫:
// 1. v_in = v_out1 + v_out2 + fee  (骞宠　)
// 2. 0 鈮?v_in < 2^64                (鑼冨洿璇佹槑)
// 3. 0 鈮?v_out1 < 2^64
// 4. 0 鈮?v_out2 < 2^64
// 5. C_in = v_in路H + r_in路G         (鎵胯鎵撳紑)
// 6. C_out1 = v_out1路H + r_out1路G
// 7. C_out2 = v_out2路H + r_out2路G

绾︽潫鏁伴噺: ~15K (3涓寖鍥磋瘉鏄?+ 鎵胯楠岃瘉)
璇佹槑鏃堕棿: ~3s
楠岃瘉鏃堕棿: ~5ms 鉁?(鍙帴鍙?
```

**瀵规瘮 Bulletproofs**:
- Groth16: 璇佹槑 128 bytes, 楠岃瘉 ~5ms
- Bulletproofs: 璇佹槑 ~700 bytes, 楠岃瘉 ~10ms
- **Groth16 鏇撮€傚悎閾句笂楠岃瘉!**

### 鍦烘櫙 2: 绉佹湁鏅鸿兘鍚堢害
```rust
// 鍏紑:
// - 鍚堢害鐘舵€佹牴: state_root
// - 鐘舵€佽浆鎹? state_root' (鏂版牴)

// 绉佹湁:
// - 鍚堢害浠ｇ爜鎵ц trace
// - 杈撳叆鍙傛暟

// 鐢佃矾: zkVM 鎵ц楠岃瘉
// 绾︽潫鏁伴噺: ~1M+ (澶嶆潅鍚堢害)
// 璇佹槑鏃堕棿: ~60s
// 楠岃瘉鏃堕棿: ~5ms 鉁?
```

---

## Trusted Setup 闂

### 椋庨櫓
濡傛灉绉樺瘑鍙傛暟 `(伪, 尾, 纬, 未, 蟿)` 娉勯湶:
- 鏀诲嚮鑰呭彲浼€犱换鎰忚瘉鏄?
- 鐮村潖鏁翠釜绯荤粺瀹夊叏鎬?

### 瑙ｅ喅鏂规: MPC Ceremony (澶氭柟璁＄畻浠紡)
```
鍙備笌鑰?1: 鐢熸垚 蟿鈧? 璁＄畻 {蟿鈧? 蟿鈧伮? ...} 鈫?鍒犻櫎 蟿鈧?
鍙備笌鑰?2: 浣跨敤涓婁竴姝ョ粨鏋? 鐢熸垚 蟿鈧? 璁＄畻 {蟿鈧伮废勨倐, (蟿鈧伮废勨倐)虏, ...} 鈫?鍒犻櫎 蟿鈧?
...
鍙備笌鑰?N: 鏈€缁?蟿 = 蟿鈧伮废勨倐路...路蟿鈧?

鍙鏈?1 涓瘹瀹炲弬涓庤€呭垹闄や簡绉樺瘑, Setup 灏辨槸瀹夊叏鐨?
```

**瀹炰緥**:
- Zcash Powers of Tau: 176 鍙備笌鑰?
- Filecoin: 2000+ 鍙備笌鑰?

### 閫氱敤 Setup (PLONK 鏀硅繘)
- Groth16: 姣忎釜鐢佃矾闇€瑕佺嫭绔?Setup
- PLONK: 涓€娆?Setup 鍙敤浜庢墍鏈夌數璺?
- **SuperVM 閫夋嫨**: 濡傛灉棰戠箒鏇存柊鐢佃矾, 鑰冭檻 PLONK

---

## 瀹炵幇搴撳姣?

### bellman (Rust)
```toml
[dependencies]
bellman = "0.14"
pairing = "0.23"  # BN254 / BLS12-381 鏇茬嚎
```

**绀轰緥浠ｇ爜**:
```rust
use bellman::{Circuit, ConstraintSystem, SynthesisError};
use pairing::bn256::{Bn256, Fr};

struct MyCircuit {
    x: Option<Fr>,  // 绉佹湁杈撳叆
}

impl Circuit<Bn256> for MyCircuit {
    fn synthesize<CS: ConstraintSystem<Bn256>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        // 鍒嗛厤鍙橀噺
        let x = cs.alloc(|| "x", || {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        // 娣诲姞绾︽潫: x虏 = x (x=0 or x=1)
        cs.enforce(
            || "x * x = x",
            |lc| lc + x,
            |lc| lc + x,
            |lc| lc + x,
        );
        
        Ok(())
    }
}
```

**浼樼偣**:
- 鎴愮啛绋冲畾 (Zcash 浣跨敤)
- 鏂囨。杈冨畬鍠?
- 鎬ц兘浼樺寲濂?

**缂虹偣**:
- API 杈冨簳灞?(鎵嬪姩鏋勫缓绾︽潫)
- Trusted Setup 澶嶆潅

### arkworks (Rust)
```toml
[dependencies]
ark-groth16 = "0.4"
ark-bn254 = "0.4"
ark-std = "0.4"
```

**浼樼偣**:
- 妯″潡鍖栬璁?
- 鏀寔澶氱璇佹槑绯荤粺 (Groth16, Marlin, GM17)
- 楂樻€ц兘

**缂虹偣**:
- 瀛︿範鏇茬嚎闄″抄
- 鏂囨。涓嶅 bellman 瀹屽杽

---

## 涓嬩竴姝ヨ鍒?

### Week 3 Day 2-3
1. 鉁?瀹屾垚 Groth16 鍘熺悊瀛︿範
2. 鉁?鍘?bellman 璺嚎灏濊瘯锛坧airing 鐗规€?妯″潡鍦?0.20/0.21/0.23 鐗堟湰涓婂瓨鍦ㄤ笉涓€鑷达紝瀵煎叆涓?API 瀵归綈鎴愭湰杈冮珮锛?
3. 鉁?鍒囨崲 arkworks 璺嚎锛氭渶灏忕數璺?a*b=c 瀹炵幇涓庢祴璇曢€氳繃锛坅rk-groth16 + ark-bls12-381锛?
4. 鉁?瀹炵幇绠€鏄?Range 璇佹槑鐢佃矾锛堜綅鍒嗚В + 甯冨皵绾︽潫锛夛紝娴嬭瘯閫氳繃
5. 鈴?杩愯鍩哄噯娴嬭瘯锛堝凡鎺ュ叆 setup/prove/verify 涓?range_prove锛岄娆¤繍琛屼腑锛?

### Week 3 Day 4-7
1. 鉁?鐮旂┒ arkworks 瀹炵幇骞跺畬鎴?PoC
2. 鈴?瀵规瘮 bellman vs arkworks 鎬ц兘锛堜互鐩稿悓鐢佃矾鍦ㄤ袱濂楀簱璺?benchmark锛?
3. 鈴?鍑嗗杩涘叆 PLONK 瀛︿範 (Week 4)

---

## 浠?bellman 杩佺Щ鍒?arkworks 鐨勫師鍥犱笌鏈€灏忕ず渚?

鍦?Windows/褰撳墠宸ュ叿閾句笅锛宲airing 0.20/0.21/0.23 鐨勬洸绾挎ā鍧楀鍑猴紙`pairing::bn256` / `pairing::bls12_381`锛変笌鐗规€у紑鍏冲瓨鍦ㄤ笉涓€鑷达紝瀵艰嚧 bellman 绀轰緥鍦ㄦ洸绾垮鍏ヤ笌 `verify_proof` 杩斿洖绫诲瀷鏈熸湜涔嬮棿闇€瑕佽緝澶氱増鏈榻愬伐浣溿€備负鍔犲揩楠岃瘉闂幆锛屾垜浠垏鎹㈠埌 arkworks 鐢熸€侊紙`ark-groth16`銆乣ark-bls12-381`锛夛紝鎺ュ彛鏇寸ǔ瀹氭竻鏅帮紝鑳藉揩閫熷緱鍒板彲鐢ㄧ殑绔埌绔粨鏋滀笌鍩哄噯銆?

鏈€灏忕數璺紙a*b=c锛夌ず渚嬩綅缃細`zk-groth16-test/src/lib.rs`
- Setup: `Groth16::<Bls12_381>::generate_random_parameters_with_reduction`
- Prove: `Groth16::<Bls12_381>::prove(&params, circuit, rng)`
- Verify: `Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c])`

Range 璇佹槑锛堜綅鍒嗚В锛夌ず渚嬩綅缃細`zk-groth16-test/src/range_proof.rs`
- 绾︽潫锛歚v = c`锛屼互鍙?`v = sum(b_i * 2^i)` 涓旀瘡涓?`b_i` 婊¤冻甯冨皵绾︽潫 `b_i*(b_i-1)=0`
- 鐢ㄤ簬婕旂ず鑼冨洿绾︽潫鐨勬瀯寤烘柟寮忥紱涓嬩竴姝ュ皢缁撳悎 Pedersen 鎵胯瀹炵幇闅愯棌鍊肩殑鑼冨洿璇佹槑銆?

---

## 鍙傝€冭祫鏂?

1. **Groth16 璁烘枃**: "On the Size of Pairing-based Non-interactive Arguments" (Eurocrypt 2016)
2. **Zero Knowledge Blog**: https://www.zeroknowledgeblog.com/index.php/groth16
3. **bellman 婧愮爜**: https://github.com/zkcrypto/bellman
4. **Zcash Sapling**: https://z.cash/upgrade/sapling/
5. **Powers of Tau**: https://github.com/ZcashFoundation/powersoftau-attestations

---

*鏈€鍚庢洿鏂? 2025-11-04*




