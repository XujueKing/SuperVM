# curve25519-dalek 瀛︿範绗旇

**鐮旂┒鍛ㄦ湡**: Week 1-2 (2025-11-04 鑷?2025-11-17)  
**瀹樻柟浠撳簱**: https://github.com/dalek-cryptography/curve25519-dalek  
**鏂囨。**: https://docs.rs/curve25519-dalek/  
**瀛︿範鐩爣**: 鎺屾彙 Ristretto Point API, 涓?Ring Signature 瀹炵幇鍋氬噯澶?

---

## 馃搵 瀛︿範娓呭崟

- [ ] RistrettoPoint 鍩虹鎿嶄綔
- [ ] Scalar 杩愮畻
- [ ] 绀轰緥浠ｇ爜杩愯
- [ ] 鎬ц兘鍩哄噯娴嬭瘯

---

## 馃敡 1. 渚濊禆娣诲姞

### 1.1 Cargo.toml 閰嶇疆

```toml
[dependencies]
curve25519-dalek = { version = "4.1", features = ["serde"] }
sha2 = "0.10"
rand = "0.8"
```

**TODO**: 鍦?Week 9 瀹炵幇闃舵娣诲姞鍒?`vm-runtime/Cargo.toml`

---

## 馃搻 2. RistrettoPoint 鍩虹

### 2.1 浠€涔堟槸 Ristretto?

**Ristretto** 鏄?Curve25519 鐨?缇ゆ娊璞″眰":
- 瑙ｅ喅 Curve25519 cofactor = 8 鐨勯棶棰?
- 鎻愪緵鍞竴缂栫爜 (姣忎釜鐐瑰彧鏈変竴绉嶈〃绀?
- 闃叉灏忓瓙缇ゆ敾鍑?

**涓轰粈涔堜笉鐢?EdwardsPoint?**
- EdwardsPoint 鏈?cofactor 闂
- 鍚屼竴涓偣鍙兘鏈夊绉嶇紪鐮?(瀹夊叏闅愭偅)
- Ristretto 淇濊瘉鍞竴鎬?+ 绱犳暟闃剁兢

### 2.2 鍩虹 API

```rust
use curve25519_dalek::ristretto::{RistrettoPoint, CompressedRistretto};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;

// 1. 鍩虹偣
let G = RISTRETTO_BASEPOINT_POINT;

// 2. 鏍囬噺涔樻硶 (鐢熸垚鍏挜)
let secret = Scalar::random(&mut rng);
let public = secret * G;

// 3. 鐐瑰姞娉?
let point1 = Scalar::random(&mut rng) * G;
let point2 = Scalar::random(&mut rng) * G;
let sum = point1 + point2;

// 4. 鍘嬬缉/瑙ｅ帇缂?(32 瀛楄妭)
let compressed: CompressedRistretto = public.compress();
let bytes: [u8; 32] = compressed.to_bytes();
let decompressed: Option<RistrettoPoint> = compressed.decompress();
```

### 2.3 绀轰緥浠ｇ爜

**TODO**: 鍒涘缓骞惰繍琛岀ず渚?

```rust
// examples/ristretto_demo.rs
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use rand::rngs::OsRng;

fn main() {
    let mut rng = OsRng;
    
    // 鐢熸垚瀵嗛挜瀵?
    let secret_key = Scalar::random(&mut rng);
    let public_key = secret_key * RISTRETTO_BASEPOINT_POINT;
    
    println!("Secret key: {:?}", secret_key);
    println!("Public key (compressed): {:?}", public_key.compress().to_bytes());
    
    // 娴嬭瘯鍚屾€佹€?(a+b)*G = a*G + b*G
    let a = Scalar::random(&mut rng);
    let b = Scalar::random(&mut rng);
    let lhs = (a + b) * RISTRETTO_BASEPOINT_POINT;
    let rhs = a * RISTRETTO_BASEPOINT_POINT + b * RISTRETTO_BASEPOINT_POINT;
    assert_eq!(lhs, rhs);
    println!("Homomorphism test passed!");
}
```

---

## 馃敘 3. Scalar 杩愮畻

### 3.1 Scalar 绫诲瀷

**Scalar = 鏁存暟 mod l** (l = 2^252 + 27742317777372353535851937790883648493)

```rust
use curve25519_dalek::scalar::Scalar;

// 1. 鐢熸垚闅忔満鏁?
let s = Scalar::random(&mut rng);

// 2. 浠庡瓧鑺傝浆鎹?
let bytes: [u8; 32] = [1u8; 32];
let s = Scalar::from_bytes_mod_order(bytes); // 妯?l 鍖栫畝

// 3. 鍥涘垯杩愮畻
let a = Scalar::random(&mut rng);
let b = Scalar::random(&mut rng);
let sum = a + b;
let diff = a - b;
let prod = a * b;
let inv = a.invert();  // 妯￠€嗗厓 (濡傛灉 a != 0)
let quot = a * inv;    // 闄ゆ硶 = a * b^(-1)

// 4. 鐗规畩鍊?
let zero = Scalar::zero();
let one = Scalar::one();
```

### 3.2 搴旂敤鍦烘櫙

**鍦?Ring Signature 涓殑鐢ㄩ€?*:
1. **绉侀挜**: SecretKey = Scalar
2. **鎸戞垬鍊?*: challenge = Scalar (Fiat-Shamir)
3. **鍝嶅簲鍊?*: response = Scalar (绛惧悕鐨勪竴閮ㄥ垎)
4. **鐩插洜瀛?*: blinding_factor = Scalar (Pedersen Commitment)

---

## 馃攼 4. Hash-to-Point

### 4.1 涓轰粈涔堥渶瑕?

**Key Image 鐢熸垚**: I = x * Hp(P)
- Hp(P) 蹇呴』鏄‘瀹氭€х殑鐐?(浠庡叕閽?P 娲剧敓)
- 涓嶈兘绠€鍗曞搱甯屽埌瀛楄妭 (闇€瑕佹槸鏇茬嚎涓婄殑鐐?

### 4.2 瀹炵幇鏂规硶

**鏂规硶 1: RistrettoPoint::from_uniform_bytes()**

```rust
use sha2::{Sha512, Digest};
use curve25519_dalek::ristretto::RistrettoPoint;

fn hash_to_point(data: &[u8]) -> RistrettoPoint {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    
    // 灏?64 瀛楄妭鍝堝笇鏄犲皠鍒扮偣
    RistrettoPoint::from_uniform_bytes(&hash.into())
}
```

**鏂规硶 2: RistrettoPoint::hash_from_bytes()** (闇€瑕?`digest` feature)

```rust
use sha2::Sha512;
use curve25519_dalek::ristretto::RistrettoPoint;

fn hash_to_point(data: &[u8]) -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(data)
}
```

### 4.3 Key Image 鐢熸垚绀轰緥

```rust
fn generate_key_image(secret_key: &Scalar, public_key: &RistrettoPoint) -> RistrettoPoint {
    // Monero: I = x * Hp(P)
    let hp = hash_to_point(&public_key.compress().to_bytes());
    secret_key * hp
}

// 楠岃瘉: 鐭ラ亾 (x, P=xG) 鍙绠?I, 浣嗕粠 (P, I) 鏃犳硶鎺ㄥ嚭 x
```

---

## 馃幆 5. Pedersen Commitment

### 5.1 鐞嗚

**鎵胯鏂规**: C(a, r) = aH + rG
- a: 閲戦 (secret)
- r: 鐩插洜瀛?(blinding factor)
- G, H: 涓や釜鐙珛鍩虹偣

**鍚屾€佹€?*: C(a1, r1) + C(a2, r2) = C(a1+a2, r1+r2)

### 5.2 瀹炵幇

```rust
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;

lazy_static! {
    // H = hash_to_point("Pedersen_H")
    static ref H: RistrettoPoint = {
        RistrettoPoint::hash_from_bytes::<Sha512>(b"Pedersen_H")
    };
}

fn commit(amount: u64, blinding_factor: &Scalar) -> RistrettoPoint {
    let amount_scalar = Scalar::from(amount);
    amount_scalar * (*H) + blinding_factor * RISTRETTO_BASEPOINT_POINT
}

// 楠岃瘉骞宠　: sum(C_inputs) = sum(C_outputs)
fn verify_balance(
    input_commitments: &[RistrettoPoint],
    output_commitments: &[RistrettoPoint],
) -> bool {
    let sum_inputs: RistrettoPoint = input_commitments.iter().sum();
    let sum_outputs: RistrettoPoint = output_commitments.iter().sum();
    sum_inputs == sum_outputs
}
```

### 5.3 搴旂敤鍒?SuperVM

**TODO**: Week 17-20 瀹炵幇 Commitment 妯″潡鏃跺弬鑰?

---

## 鈿?6. 鎬ц兘鍩哄噯

### 6.1 鍩虹鎿嶄綔鎬ц兘

**TODO**: 杩愯 `cargo bench` 娴嬭瘯

棰勬湡鎬ц兘 (Intel i7, 鍗曟牳):
- Scalar 涔樻硶: ~50-60 渭s
- Point 鍔犳硶: ~10 渭s
- Point 鍘嬬缉: ~5 渭s
- Point 瑙ｅ帇缂? ~60 渭s
- Hash-to-point: ~80 渭s

### 6.2 浼樺寲鎶€宸?

**鎵归噺鎿嶄綔**:
```rust
// 鎱? 閫愪釜璁＄畻
let mut sum = RistrettoPoint::identity();
for scalar in scalars {
    sum += scalar * RISTRETTO_BASEPOINT_POINT;
}

// 蹇? 浣跨敤 Straus 绠楁硶 (澶氭爣閲忎箻娉?
use curve25519_dalek::traits::VartimeMultiscalarMul;
let sum = RistrettoPoint::vartime_multiscalar_mul(
    &scalars,
    &points
);
```

**棰勮绠楄〃**:
```rust
use curve25519_dalek::traits::MultiscalarMul;

// 棰勮绠楀熀鐐硅〃 (鍔犻€熼噸澶嶄箻娉?
let precomputed = RISTRETTO_BASEPOINT_POINT.precompute();
let result = precomputed.multiply(&scalar);
```

---

## 馃И 7. 瀹炴垬缁冧範

### 7.1 缁冧範 1: Ed25519 绛惧悕

**TODO**: 瀹炵幇绠€鍗曠殑 Schnorr 绛惧悕

```rust
// Schnorr 绛惧悕: (R, s) where R = rG, s = r + H(R||P||m)*x
struct SchnorrSignature {
    R: RistrettoPoint,
    s: Scalar,
}

fn sign(message: &[u8], secret_key: &Scalar) -> SchnorrSignature {
    // TODO: 瀹炵幇
    todo!()
}

fn verify(message: &[u8], public_key: &RistrettoPoint, sig: &SchnorrSignature) -> bool {
    // TODO: 楠岃瘉 sG = R + H(R||P||m)*P
    todo!()
}
```

### 7.2 缁冧範 2: 绠€鍗?Ring Signature

**鐩爣**: 瀹炵幇 2-of-3 鐜鍚?(绠€鍖栫増)

```rust
struct SimpleRingSignature {
    ring: [RistrettoPoint; 3],
    c: [Scalar; 3],      // 鎸戞垬鍊?
    r: [Scalar; 3],      // 鍝嶅簲鍊?
}

// TODO: 瀹炵幇 sign() 鍜?verify()
```

---

## 馃摎 8. 鍙傝€冭祫鏂?

### 8.1 瀹樻柟鏂囨。

- **curve25519-dalek docs**: https://docs.rs/curve25519-dalek/
- **Ristretto 璁烘枃**: https://ristretto.group/
- **Mike Hamburg's paper**: https://eprint.iacr.org/2015/673

### 8.2 绀轰緥椤圭洰

- **bulletproofs**: https://github.com/dalek-cryptography/bulletproofs
  - 浣跨敤 curve25519-dalek 瀹炵幇鑼冨洿璇佹槑
- **ed25519-dalek**: https://github.com/dalek-cryptography/ed25519-dalek
  - EdDSA 绛惧悕瀹炵幇

### 8.3 瀛︿範璺緞

**鎺ㄨ崘椤哄簭**:
1. 闃呰 docs.rs API 鏂囨。 (2-3 灏忔椂)
2. 杩愯瀹樻柟绀轰緥 (1 灏忔椂)
3. 瀹炵幇 Schnorr 绛惧悕 (鍗婂ぉ)
4. 瀹炵幇绠€鍗?Ring Signature (1 澶?
5. 鐮旂┒ bulletproofs 婧愮爜 (2-3 澶?

---

## 鉁?瀛︿範杩涘害

### Week 1 (2025-11-04 鑷?2025-11-10)

**Day 1 (2025-11-04)**:
- [x] 鍒涘缓瀛︿範绗旇
- [ ] 娣诲姞渚濊禆鍒版祴璇曢」鐩?
- [ ] 杩愯鍩虹绀轰緥 (RistrettoPoint, Scalar)

**Day 2-3**:
- [ ] 瀹炵幇 Hash-to-Point
- [ ] 瀹炵幇 Pedersen Commitment
- [ ] 鎬ц兘鍩哄噯娴嬭瘯

**Day 4-5**:
- [ ] 瀹炵幇 Schnorr 绛惧悕
- [ ] 瀹炵幇绠€鍗?Ring Signature (2-of-3)
- [ ] 鍗曞厓娴嬭瘯

**Day 6-7**:
- [ ] 闃呰 bulletproofs 婧愮爜
- [ ] 鎬荤粨瀛︿範鎴愭灉
- [ ] 鍑嗗 Week 2 楂樼骇涓婚

### Week 2 (2025-11-11 鑷?2025-11-17)

**Day 8-10**:
- [ ] 娣卞叆澶氭爣閲忎箻娉曚紭鍖?
- [ ] 鎵归噺楠岃瘉鎶€鏈?
- [ ] 鍐呭瓨瀹夊叏瀹炶返 (zeroize)

**Day 11-14**:
- [ ] 缁撳悎 Monero 婧愮爜鐞嗚В搴旂敤
- [ ] 璁捐 SuperVM Ring Signature API
- [ ] 缂栧啓鎶€鏈€夊瀷鎶ュ憡

---

## 馃挕 闂涓庢€濊€?

### 甯歌闂

1. **RistrettoPoint vs EdwardsPoint?**
   - 绛? 姘歌繙鐢?Ristretto (闄ら潪浣犵煡閬撹嚜宸卞湪鍋氫粈涔?

2. **鎬ц兘鐡堕鍦ㄥ摢?**
   - 绛? Scalar 涔樻硶 (鍗?80% 鏃堕棿)

3. **濡備綍瀹夊叏澶勭悊绉侀挜?**
   - 绛? 浣跨敤 `zeroize` crate 鑷姩娓呴浂

### 涓汉鐞嗚В

**TODO**: 姣忔棩璁板綍瀛︿範蹇冨緱

---

**鏈€鍚庢洿鏂?*: 2025-11-04  
**涓嬫鏇存柊**: 姣忔棩鍚屾瀛︿範杩涘害




