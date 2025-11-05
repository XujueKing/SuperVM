# zkSNARK 鎶€鏈瘎浼版姤鍛?
**Phase 2 Week 3-4**

## 鐩爣

璇勪及涓绘祦 zkSNARK 搴撳湪 SuperVM Privacy Layer 涓殑閫傜敤鎬э紝涓?Week 5-8 鐨勯浂鐭ヨ瘑璇佹槑瀹炵幇鎻愪緵鎶€鏈€夊瀷渚濇嵁銆?

## 寰呰瘎浼板簱

### 1. bellman (Zcash - Groth16)
- **椤圭洰**: https://github.com/zkcrypto/bellman
- **璇佹槑绯荤粺**: Groth16
- **鐗圭偣**: 
  - 璇佹槑澶у皬: 鍥哄畾 ~128 bytes (3涓兢鍏冪礌)
  - 楠岃瘉鏃堕棿: 鍥哄畾 ~5ms (3涓厤瀵硅繍绠?
  - Trusted Setup: 闇€瑕?(涓€娆℃€? 浣嗛渶瑕佹瘡涓數璺崟鐙?Setup)
  - 鎴愮啛搴? 楂?(Zcash Sapling 浣跨敤)

### 2. plonky2 (Polygon Zero - PLONK)
- **椤圭洰**: https://github.com/0xPolygonZero/plonky2
- **璇佹槑绯荤粺**: PLONK (Permutation-based)
- **鐗圭偣**:
  - 璇佹槑澶у皬: ~10KB (鍙栧喅浜庣數璺ぇ灏?
  - 楠岃瘉鏃堕棿: ~10-50ms
  - Trusted Setup: 闇€瑕?(浣嗛€氱敤, 涓€娆?Setup 鍙敤浜庢墍鏈夌數璺?
  - 鎴愮啛搴? 楂?(Polygon zkEVM 浣跨敤)

### 3. halo2 (Electric Coin Company - Halo2)
- **椤圭洰**: https://github.com/zcash/halo2
- **璇佹槑绯荤粺**: Halo 2 (閫掑綊 SNARK)
- **鐗圭偣**:
  - 璇佹槑澶у皬: ~50KB (閫掑綊璇佹槑)
  - 楠岃瘉鏃堕棿: ~100-200ms
  - Trusted Setup: **涓嶉渶瑕?* (閫忔槑 Setup)
  - 鎴愮啛搴? 涓?(Zcash Orchard 鍗囩骇浣跨敤)

### 4. arkworks (閫氱敤 zkSNARK 宸ュ叿搴?
- **椤圭洰**: https://github.com/arkworks-rs/
- **璇佹槑绯荤粺**: Groth16, Marlin, GM17 绛夊绉?
- **鐗圭偣**:
  - 妯″潡鍖栬璁? 鍙粍鍚堜笉鍚岃瘉鏄庣郴缁?
  - 鎬ц兘浼樺寲鑹ソ
  - 鏂囨。杈冨畬鍠?

### 5. halo2 (Zcash)
- **椤圭洰**: https://github.com/zcash/halo2
- **璇佹槑绯荤粺**: Halo 2锛堟洿閫氱敤鐨?PLONK 鍙樹綋锛屾敮鎸侀€掑綊锛?
- **鐗圭偣**:
  - 閫忔槑鎴栭€氱敤 Setup锛堟洿鐏垫椿锛?
  - 鍘熺敓閫掑綊鍙嬪ソ
  - API 鍋忓簳灞傦紙闇€瑕佽嚜瀹氫箟 Gate/Chip锛夛紝瀛︿範鏇茬嚎杈冮櫋

---

## 璇勪及缁村害

### 1. 鎬ц兘瀵规瘮

| 鎸囨爣 | bellman (Groth16) | plonky2 (PLONK) | halo2 (Halo2) | arkworks (Groth16) |
|------|-------------------|------------------|---------------|---------------------|
| 璇佹槑鐢熸垚鏃堕棿 | ~2-5s | ~5-20s | ~10-30s | ~2-5s |
| 楠岃瘉鏃堕棿 | ~5ms | ~10-50ms | ~100-200ms | ~5ms |
| 璇佹槑澶у皬 | ~128 bytes | ~10KB | ~50KB | ~128 bytes |
| Trusted Setup | 闇€瑕?姣忕數璺? | 闇€瑕?閫氱敤) | 涓嶉渶瑕?| 闇€瑕?姣忕數璺? |
| 鍐呭瓨鍗犵敤 | 涓?(~2GB) | 楂?(~8GB) | 楂?(~16GB) | 涓?(~2GB) |

### 2. API 澶嶆潅搴?

#### bellman 绀轰緥 (Groth16)
```rust
use bellman::{Circuit, ConstraintSystem, SynthesisError};
use pairing::bn256::{Bn256, Fr};

struct MyCircuit {
    x: Option<Fr>,
}

impl<E: Engine> Circuit<E> for MyCircuit {
    fn synthesize<CS: ConstraintSystem<E>>(
        self,
        cs: &mut CS
    ) -> Result<(), SynthesisError> {
        // 绾︽潫鏋勫缓浠ｇ爜
        let x = cs.alloc(|| "x", || {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        // x * x = x^2
        cs.enforce(
            || "x^2",
            |lc| lc + x,
            |lc| lc + x,
            |lc| lc + x_squared,
        );
        
        Ok(())
    }
}
```

**澶嶆潅搴?*: 猸愨瓙猸?(涓瓑, 闇€瑕佹墜鍔ㄦ瀯寤?R1CS 绾︽潫)

#### plonky2 绀轰緥 (PLONK)
```rust
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = <C as GenericConfig<D>>::F;

let mut builder = CircuitBuilder::<F, D>::new(config);
let x = builder.add_virtual_target();
let y = builder.add_virtual_target();

// x + y = z
let z = builder.add(x, y);
builder.register_public_input(z);

let data = builder.build::<C>();
```

**澶嶆潅搴?*: 猸愨瓙 (杈冧綆, Builder 妯″紡, 浣嗙被鍨嬪弬鏁板鏉?

#### halo2 绀轰緥 (Halo2)
```rust
use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error},
};

#[derive(Clone)]
struct MyConfig {
    advice: Column<Advice>,
}

struct MyCircuit {
    x: Value<Fp>,
}

impl Circuit<Fp> for MyCircuit {
    type Config = MyConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        let advice = meta.advice_column();
        
        meta.create_gate("x^2", |meta| {
            let x = meta.query_advice(advice, Rotation::cur());
            vec![x.clone() * x.clone()]
        });
        
        MyConfig { advice }
    }
    
    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
        // 鐢佃矾瀹炵幇
        Ok(())
    }
}
```

**澶嶆潅搴?*: 猸愨瓙猸愨瓙 (楂? 闇€瑕佺悊瑙?Layouter, Rotation, Gate 绛夋蹇?

### 3. 绀惧尯鏀寔涓庣敓鎬?

| 椤圭洰 | GitHub Stars | 鏈€杩戞洿鏂?| 鐢熶骇搴旂敤 | 鏂囨。璐ㄩ噺 | Rust 鐢熸€?|
|------|--------------|----------|----------|----------|-----------|
| bellman | 700+ | 娲昏穬 | Zcash Sapling | 猸愨瓙猸?| 涓?|
| plonky2 | 1.5K+ | 娲昏穬 | Polygon zkEVM | 猸愨瓙猸愨瓙 | 楂?|
| halo2 | 700+ | 娲昏穬 | Zcash Orchard | 猸愨瓙猸愨瓙猸?| 楂?|
| arkworks | 700+ | 娲昏穬 | Aleo, Mina | 猸愨瓙猸愨瓙 | 楂?|

### 4. SuperVM 閫傞厤鎬у垎鏋?

#### 鍦烘櫙 1: 闅愯棌鍚堢害鐘舵€?(RingCT + zkSNARK)
- **闇€姹?*: 璇佹槑 `危杈撳叆 = 危杈撳嚭 + 鎵嬬画璐筦 涓旀墍鏈夐噾棰?鈭?[0, 2^64-1]
- **绾︽潫瑙勬ā**: ~10K 绾︽潫 (Bulletproofs 绾?5K 绾︽潫)
- **鎺ㄨ崘**: **bellman (Groth16)** 鎴?**arkworks (Groth16)**
  - 鐞嗙敱: 楠岃瘉鏃堕棿 ~5ms, 璇佹槑澶у皬 128 bytes, 閫傚悎閾句笂楠岃瘉
  - Trusted Setup 鍙帴鍙?(涓€娆℃€? 鐢辩ぞ鍖哄畬鎴?

#### 鍦烘櫙 2: 閫氱敤鏅鸿兘鍚堢害闅愮 (zkVM)
- **闇€姹?*: 璇佹槑鍚堢害鎵ц姝ｇ‘鎬? 鏀寔澶嶆潅閫昏緫
- **绾︽潫瑙勬ā**: ~1M+ 绾︽潫
- **鎺ㄨ崘**: **plonky2 (PLONK)** 鎴?**halo2 (Halo2)**
  - 鐞嗙敱: 
    - plonky2: 閫氱敤 Setup, 閫傚悎棰戠箒鏇存柊鐢佃矾
    - halo2: 鏃?Trusted Setup, 閫掑綊璇佹槑鏀寔澶х數璺?

#### 鍦烘櫙 3: 璺ㄩ摼闅愮妗?(閫掑綊璇佹槑)
- **闇€姹?*: 鑱氬悎澶氫釜璇佹槑, 鍑忓皯閾句笂楠岃瘉鎴愭湰
- **鎺ㄨ崘**: **halo2 (Halo2)**
  - 鐞嗙敱: 鍘熺敓鏀寔閫掑綊, 鍙皢 N 涓瘉鏄庤仛鍚堜负 1 涓?

---

## 鎬ц兘鍩哄噯娴嬭瘯璁″垝涓庡垵姝ョ粨鏋?

### 褰撳墠瀹炵幇涓庣幆澧?
- 搴撲笌鏇茬嚎锛歛rkworks锛坅rk-groth16 0.4 + ark-bls12-381 0.4锛?
- 骞冲彴锛歐indows锛圥owerShell锛夛紝Rust stable
- 宸插疄鐜扮數璺細
  - Multiply锛坅*b=c锛夛細鍏紑 c锛涚害鏉?1 涓箻娉曢棬
  - Range锛堜綅鍒嗚В锛?-bit锛夛細鍏紑 c=v锛涚害鏉熻嫢骞插竷灏旈棬 + 绾挎€х害鏉?
- 浠ｇ爜浣嶇疆锛歚zk-groth16-test/`

### Groth16 鍩哄噯锛坅rkworks 0.4 + BLS12-381锛?

| 鎿嶄綔 | 骞冲潎鑰楁椂 | 绾︽潫鏁?| 璇存槑 |
|------|---------|-------|------|
| multiply_setup | 31.1 ms | ~1 | Trusted Setup锛?涓箻娉曠害鏉燂級|
| multiply_prove | 5.2 ms | ~1 | 璇佹槑鐢熸垚锛坅=3, b=5, c=15锛墊
| multiply_verify | 3.6 ms | ~1 | 楠岃瘉璇佹槑锛堝崟涓叕寮€杈撳叆锛墊
| range_8bit_prove | 4.4 ms | ~10 | 8-bit 鑼冨洿璇佹槑鐢熸垚锛?2 < 2^8锛墊
| **range_64bit_setup** | **19.6 ms** | **~70** | **64-bit 鑼冨洿璇佹槑 Trusted Setup** |
| **range_64bit_prove** | **7.4 ms** | **~70** | **64-bit 鑼冨洿璇佹槑鐢熸垚锛坴=12345678901234锛?* |
| pedersen_prove | 3.8 ms | ~2 | Pedersen 鎵胯璇佹槑锛坴=100, r=42锛墊
| combined_setup | 26.8 ms | ~72 | Pedersen + 64-bit Range 缁勫悎鐢佃矾 Setup |
| combined_prove | 10.0 ms | ~72 | Pedersen + 64-bit Range 缁勫悎鐢佃矾 Prove |

**鐜**锛歐indows 10/PowerShell锛孯ust 1.x release 浼樺寲锛坄--release`锛夛紝寮€鍙戞満锛堝浠诲姟鑳屾櫙锛夈€?

**鍏抽敭瑙傚療**锛?
1. **Setup 鏃堕棿**锛氱害鏉熸暟浠?1 鈫?70锛宻etup 鏃堕棿浠?31ms 鈫?19.6ms锛堜紭鍖栧悗鏇寸ǔ瀹氾級
2. **璇佹槑鐢熸垚鎵╁睍鎬т紭寮?*锛氱害鏉熸暟 脳7锛?0 鈫?70锛夛紝璇佹槑鏃堕棿浠?脳1.7锛?.4ms 鈫?7.4ms锛?*浜氱嚎鎬у闀?*鉁?
3. **楠岃瘉鏃堕棿**锛氭亽瀹?~3.6ms锛圙roth16 鐗规€э細3娆￠厤瀵癸紝涓嶉殢鐢佃矾澶嶆潅搴﹀闀匡級鉁?
4. **璇佹槑澶у皬**锛?28 bytes锛?脳G1 + 1脳G2锛屾亽瀹氾級

**涓庣悊璁洪鏈熷姣?*锛?
- 楠岃瘉鏃堕棿绗﹀悎棰勬湡锛垀3.6ms锛夛紝鎺ヨ繎鐞嗚鍊硷紙3娆￠厤瀵癸級
- 璇佹槑澶у皬绗﹀悎棰勬湡锛堟亽瀹?128 bytes锛?
- Setup 涓庤瘉鏄庢椂闂村湪姝ｅ父鑼冨洿鍐咃紙寰绾ц繍绠?脳 绾︽潫鏁?+ 閰嶅寮€閿€锛?

**64-bit vs 8-bit 鑼冨洿璇佹槑鎬ц兘鍒嗘瀽**锛?
| 鎸囨爣 | 8-bit | 64-bit | 澧為暱姣斾緥 | 鍒嗘瀽 |
|------|-------|--------|----------|------|
| 绾︽潫鏁?| ~10 | ~70 | 脳7.0 | 浣嶅垎瑙ｇ嚎鎬у闀?|
| Setup 鏃堕棿 | N/A | 19.6 ms | - | 鐙珛 setup |
| Prove 鏃堕棿 | 4.4 ms | 7.4 ms | 脳1.7 | **浜氱嚎鎬у闀?*鉁?|
| Verify 鏃堕棿 | ~3.6 ms | ~3.6 ms | 脳1.0 | **鎭掑畾鏃堕棿**鉁?|
| 璇佹槑澶у皬 | 128 bytes | 128 bytes | 脳1.0 | **鎭掑畾澶у皬**鉁?|

**鍏抽敭鍙戠幇**锛?
- **Groth16 鎵╁睍鎬ч獙璇佹垚鍔?*锛氱害鏉熸暟 7 鍊嶅闀匡紝璇佹槑鏃堕棿浠?1.7 鍊嶅闀?
- **楠岃瘉鎴愭湰鍥哄畾**锛氭棤璁?8-bit 杩樻槸 64-bit锛岄獙璇佹椂闂存亽瀹?~3.6ms锛岃瘉鏄庡ぇ灏忔亽瀹?128 bytes
- **瀹炵敤鎬ц瘎浼?*锛?4-bit 鑼冨洿璇佹槑锛堢湡瀹炲満鏅墍闇€锛塸rove 鏃堕棿浠?7.4ms锛屽畬鍏ㄦ弧瓒崇敓浜ч渶姹?

**鍚庣画浼樺寲鏂瑰悜**锛?
- 鍦?Linux/鍥哄畾纭欢鐜閲嶆祴锛屽噺灏戣皟搴︽姈鍔?
- 鎵归噺楠岃瘉浼樺寲锛堝涓瘉鏄庡悎骞堕獙璇佸彲鍧囨憡閰嶅鎴愭湰锛?
- 鑰冭檻 GPU 鍔犻€燂紙MSM/閰嶅杩愮畻锛?

娉細寮€鍙戞満闈炰弗璋ㄦ祴璇曠幆澧冿紝鏁版嵁浠呬緵鏁伴噺绾у弬鑰冿紱鐢熶骇閮ㄧ讲闇€鍦ㄧ洰鏍囧钩鍙伴噸娴嬨€?

### 宸插疄鐜扮數璺紙zk-groth16-test 椤圭洰锛夆渽

1. **MultiplyCircuit**锛氭渶灏忕ず渚嬶紙a*b=c锛夛紝~1 涓箻娉曠害鏉?
2. **RangeProofCircuit (8-bit)**锛?-bit 鑼冨洿璇佹槑锛堜綅鍒嗚В + 甯冨皵绾︽潫锛夛紝~10 涓害鏉?
3. **RangeProofCircuit (64-bit)**锛?4-bit 鑼冨洿璇佹槑锛堝畬鏁撮噾棰濊寖鍥达級锛寏70 涓害鏉?鉁?
4. **PedersenCommitmentCircuit**锛氱畝鍖栫嚎鎬ф壙璇猴紙C = v + r*k锛夛紝~2 涓害鏉?
   - 娉細瀹屾暣 Pedersen 闇€妞渾鏇茬嚎缇よ繍绠楋紱褰撳墠鐢ㄥ煙涔樻硶妯℃嫙绾挎€ф壙璇?

**娴嬭瘯鐘舵€?*锛?
- 鎵€鏈?4 涓數璺祴璇曢€氳繃 鉁?
- 鎵€鏈?7 涓熀鍑嗘祴璇曞畬鎴?鉁?
- 鎬ц兘鏁版嵁宸叉敹闆嗗苟楠岃瘉 鉁?

### Halo2 瀹炵幇涓庢€ц兘鏁版嵁锛坔alo2-eval 椤圭洰锛夆渽
- Crate锛歚halo2-eval/`锛坔alo2_proofs 0.3 + halo2curves 0.6锛?
- 鐢佃矾锛歁ultiply锛坅*b=c锛夛紝浣跨敤 PLONK-style Gate
- 娴嬭瘯锛歁ockProver 閫氳繃 + KZG 鐪熷疄璇佹槑/楠岃瘉閫氳繃

**Halo2 鍩哄噯锛圞ZG/Bn256锛?*锛?

| k鍊?| 鐢佃矾琛屾暟 | Setup+Keygen | Prove | Verify | 璇佹槑澶у皬 |
|-----|---------|--------------|-------|--------|----------|
| 6 | 64 | 49.5ms | 50.6ms | 3.3ms | 1600 bytes |
| 8 | 256 | 85.6ms | 106.2ms | 4.8ms | 1728 bytes |
| 10 | 1024 | 431.3ms | 186.8ms | 10.1ms | 1856 bytes |

**涓?Groth16 瀵规瘮锛圕ombined 鐢佃矾锛寏72 绾︽潫锛?*锛?
- Groth16: Setup=26.8ms | Prove=10.0ms | Verify=3.6ms | 璇佹槑澶у皬=128 bytes
- Halo2 (k=8): Setup+Keygen=85.6ms | Prove=106.2ms | Verify=4.8ms | 璇佹槑澶у皬=1728 bytes

**鍏抽敭瑙傚療**锛?
1. **璇佹槑澶у皬**锛欻alo2 璇佹槑 ~1.7KB锛坘=8锛夛紝Groth16 浠?128 bytes 鈫?**Groth16 灏?13.5脳** 鉁?
2. **楠岃瘉鏃堕棿**锛欻alo2 ~4.8ms锛坘=8锛夛紝Groth16 ~3.6ms 鈫?**Groth16 蹇?1.3脳**
3. **璇佹槑鏃堕棿**锛欻alo2 ~106ms锛坘=8锛夛紝Groth16 ~10ms 鈫?**Groth16 蹇?10.6脳** 鉁?
4. **Setup鐏垫椿鎬?*锛欻alo2 閫氱敤 Setup锛堜竴娆″彲鐢ㄤ簬浠绘剰鐢佃矾锛夛紝Groth16 闇€涓烘瘡涓數璺崟鐙?Setup 鈫?**Halo2 浼樺娍**
5. **鎵╁睍鎬?*锛欻alo2 璇佹槑鏃堕棿闅?k 鍊硷紙鐢佃矾澶嶆潅搴︼級澧為暱杈冨揩锛孏roth16 鏇寸ǔ瀹?

### Pedersen + Range 缁勫悎鐢佃矾锛堜笅涓€姝ワ級
- **鐩爣**: 闅愯棌閲戦鐨勮寖鍥磋瘉鏄?
- **鍏紑**: 鎵胯 `C = v*H + r*G`
- **绉佹湁**: 閲戦 `v 鈭?[0, 2^64-1]`, 鐩插寲鍥犲瓙 `r`
- **绾︽潫**: 
  1. 鎵胯鎵撳紑姝ｇ‘锛坄C` 璁＄畻楠岃瘉锛?
  2. 鑼冨洿妫€鏌ワ紙64-bit 浣嶅垎瑙?+ 64 涓竷灏旂害鏉燂級
- **棰勪及**: ~72 绾︽潫锛?涓壙璇虹害鏉?+ 70涓寖鍥寸害鏉燂級锛岃瘉鏄庢椂闂?~7-8ms锛堝熀浜?64-bit range 瀹炴祴锛?

### 娴嬭瘯鎸囨爣
1. 璇佹槑鐢熸垚鏃堕棿锛堝崟/鎵归噺锛?
2. 楠岃瘉鏃堕棿锛堝崟/鎵归噺锛?
3. 璇佹槑澶у皬锛圙roth16 鎭掑畾 128 bytes锛?
4. 鍐呭瓨鍗犵敤宄板€?

### 瀹炴柦姝ラ
1. **Week 3**: 
   - 鐮旂┒ Groth16 鍘熺悊 (R1CS, QAP, 閰嶅)
   - 瀹炵幇 bellman 鍩哄噯娴嬭瘯
   - 瀹炵幇 arkworks 鍩哄噯娴嬭瘯
2. **Week 4**:
   - 鐮旂┒ PLONK 鍘熺悊 (缃崲璇佹槑, KZG 鎵胯)
   - 瀹炵幇 plonky2 鍩哄噯娴嬭瘯
   - 鐮旂┒ Halo2 鍘熺悊 (閫掑綊 SNARK, IPA)
   - 缁煎悎瀵规瘮, 浜у嚭閫夊瀷鎶ュ憡

---

## 鍙傝€冭祫鏂?

### Groth16
- 璁烘枃: "On the Size of Pairing-based Non-interactive Arguments" (2016)
- 鏁欑▼: https://www.zeroknowledgeblog.com/index.php/groth16

### PLONK
- 璁烘枃: "PLONK: Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge" (2019)
- 鏁欑▼: https://vitalik.ca/general/2019/09/22/plonk.html

### Halo2
- 璁烘枃: "Recursive Proof Composition without a Trusted Setup" (2019)
- 鏂囨。: https://zcash.github.io/halo2/

### Bulletproofs (瀵规瘮鍩哄噯)
- 璁烘枃: "Bulletproofs: Short Proofs for Confidential Transactions and More" (2018)
- 鎴戜滑鐨勫疄鐜? `docs/research/monero-study-notes.md` (Bulletproofs 绔犺妭)

---

## 鎶€鏈€夊瀷缁撹 鉁?

### Groth16 vs Halo2 鍏ㄩ潰瀵规瘮

| 缁村害 | Groth16 (arkworks) | Halo2 (halo2_proofs) | 浼樺娍鏂?|
|------|-------------------|----------------------|--------|
| **璇佹槑澶у皬** | 128 bytes锛堟亽瀹氾級 | ~1.7KB锛坘=8锛?| 鉁?Groth16锛堝皬 13.5脳锛?|
| **楠岃瘉鏃堕棿** | 3.6ms锛堟亽瀹氾級 | 4.8ms锛坘=8锛?| 鉁?Groth16锛堝揩 1.3脳锛?|
| **璇佹槑鏃堕棿** | 10.0ms | 106.2ms锛坘=8锛?| 鉁?Groth16锛堝揩 10.6脳锛?|
| **Setup 鐏垫椿鎬?* | 闇€涓烘瘡涓數璺崟鐙?Setup | 閫氱敤 Setup锛堜竴娆″嵆鍙級 | 鉁?Halo2 |
| **Trusted Setup** | 闇€瑕?| 涓嶉渶瑕侊紙閫忔槑锛?| 鉁?Halo2 |
| **閫掑綊璇佹槑** | 涓嶅師鐢熸敮鎸?| 鍘熺敓鏀寔 | 鉁?Halo2 |
| **閾句笂楠岃瘉鎴愭湰** | 鏋佷綆锛圙as 鍙嬪ソ锛?| 杈冮珮 | 鉁?Groth16 |
| **寮€鍙戝鏉傚害** | 涓紙R1CS 绾︽潫锛?| 楂橈紙鑷畾涔?Gate/Chip锛?| 鉁?Groth16 |
| **鐢熸€佹垚鐔熷害** | 楂橈紙Zcash, Filecoin锛?| 涓紙Zcash Orchard锛?| 鉁?Groth16 |

### SuperVM 鍦烘櫙鎺ㄨ崘

#### 鍦烘櫙 1: 閾句笂闅愮浜ゆ槗锛圧ingCT锛夆渽 **鎺ㄨ崘 Groth16**
- **闇€姹?*: 璇佹槑閲戦鑼冨洿 + 鎵胯姝ｇ‘鎬?
- **浼樺厛绾?*: 璇佹槑澶у皬銆侀獙璇佹椂闂淬€丟as 鎴愭湰
- **缁撹**: Groth16 璇佹槑灏?13.5脳銆侀獙璇佸揩 1.3脳锛岄潪甯搁€傚悎閾句笂楠岃瘉
- **瀹炴柦**: 浣跨敤 arkworks 鐢熸€侊紝澶嶇敤宸插疄鐜扮殑 Combined 鐢佃矾

#### 鍦烘櫙 2: 璺ㄩ摼闅愮妗ワ紙閫掑綊璇佹槑锛夆渽 **鎺ㄨ崘 Halo2**
- **闇€姹?*: 鑱氬悎澶氫釜璇佹槑锛屽噺灏戦摼涓婇獙璇佹鏁?
- **浼樺厛绾?*: 閫掑綊鑳藉姏銆丼etup 鐏垫椿鎬?
- **缁撹**: Halo2 鍘熺敓鏀寔閫掑綊锛屽彲灏?N 涓瘉鏄庤仛鍚堜负 1 涓?
- **瀹炴柦**: 浣跨敤 halo2_proofs锛屽疄鐜伴€掑綊鐢佃矾

#### 鍦烘櫙 3: 棰戠箒鏇存柊鐢佃矾锛坺kVM 寮€鍙戯級鉁?**鎺ㄨ崘 Halo2**
- **闇€姹?*: 鐢佃矾蹇€熻凯浠ｏ紝閬垮厤姣忔閲嶆柊 Setup
- **浼樺厛绾?*: 寮€鍙戞晥鐜囥€丼etup 鐏垫椿鎬?
- **缁撹**: Halo2 閫氱敤 Setup锛屼竴娆″嵆鍙敤浜庝换鎰忕數璺?
- **瀹炴柦**: 浣跨敤 halo2_proofs锛屽缓绔嬬數璺簱

#### 鍦烘櫙 4: 娣峰悎绛栫暐锛堟渶浼樻柟妗堬級鉁?
- **閾句笂閮ㄥ垎**: 浣跨敤 Groth16锛堣瘉鏄庡皬銆侀獙璇佸揩銆丟as 浣庯級
- **閾句笅鑱氬悎**: 浣跨敤 Halo2 閫掑綊锛堣仛鍚堝涓?Groth16 璇佹槑锛?
- **寮€鍙戣凯浠?*: 浣跨敤 Halo2锛堝揩閫熷師鍨嬶級鈫?鐢熶骇浼樺寲涓?Groth16

### 鎺ㄨ崘鎶€鏈爤
1. **Phase 2 鍘熷瀷 (Week 5-8)**: **arkworks (Groth16)**
   - 浼樺娍: 璇佹槑灏?(128 bytes), 楠岃瘉蹇?(~5ms), 鎴愮啛绋冲畾
   - 鍔ｅ娍: Trusted Setup (鍙帴鍙?
   
2. **Phase 3 鐢熶骇 (Week 13+)**: **plonky2 (PLONK)** 鎴?**halo2 (Halo2)**
   - plonky2: 閫氱敤 Setup, 閫傚悎蹇€熻凯浠?
   - halo2: 鏃?Trusted Setup, 閫傚悎闀挎湡杩愯

### 鎬ц兘棰勬湡
- Groth16: 璇佹槑鐢熸垚 ~3s, 楠岃瘉 ~5ms, 璇佹槑澶у皬 128 bytes
- PLONK: 璇佹槑鐢熸垚 ~10s, 楠岃瘉 ~30ms, 璇佹槑澶у皬 ~10KB
- Halo2: 璇佹槑鐢熸垚 ~20s, 楠岃瘉 ~150ms, 璇佹槑澶у皬 ~50KB

**瀵规瘮 Bulletproofs** (Monero 浣跨敤):
- Bulletproofs: 璇佹槑鐢熸垚 ~50ms, 楠岃瘉 ~10ms, 璇佹槑澶у皬 ~700 bytes
- zkSNARK 浼樺娍: 楠岃瘉鏇村揩 (5ms vs 10ms), 鍙€氱敤鍖?
- zkSNARK 鍔ｅ娍: 璇佹槑鐢熸垚鎱?(3s vs 50ms)

---

## 涓嬩竴姝ヨ鍔?

1. 鉁?鍒涘缓璇勪及妗嗘灦鏂囨。 (褰撳墠鏂囦欢)
2. 鈴?鐮旂┒ Groth16 鍘熺悊涓?R1CS 绾︽潫绯荤粺
3. 鈴?鎼缓 bellman 娴嬭瘯椤圭洰
4. 鈴?瀹炵幇 Pedersen Commitment 鐢佃矾 (bellman)
5. 鈴?杩愯鍩哄噯娴嬭瘯骞惰褰曟暟鎹?
6. 鈴?閲嶅姝ラ 3-5 (arkworks, plonky2)
7. 鈴?浜у嚭鏈€缁堟妧鏈€夊瀷鎶ュ憡

---

*鏈枃妗ｉ殢鐮旂┒杩涘睍鎸佺画鏇存柊*  
*鏈€鍚庢洿鏂? 2025-11-04*




