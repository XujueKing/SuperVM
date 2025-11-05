# CryptoNote 鐧界毊涔﹀涔犵瑪璁?

**璁烘枃鏍囬**: CryptoNote v1.0  
**浣滆€?*: Nicolas van Saberhagen  
**鏃ユ湡**: 2013-10-17  
**涓嬭浇閾炬帴**: https://cryptonote.org/whitepaper.pdf  
**瀛︿範鍛ㄦ湡**: Week 1-2 (2025-11-04 鑷?2025-11-17)

---

## 馃搵 瀛︿範娓呭崟

- [ ] 绗?1-2 绔? 鑳屾櫙涓庣洰鏍?
- [ ] 绗?3 绔? Untraceable Transactions (涓嶅彲杩借釜浜ゆ槗)
- [ ] 绗?4 绔? One-time Keys (涓€娆℃€у瘑閽?
- [ ] 绗?5 绔? Ring Signatures (鐜鍚?
- [ ] 绗?6 绔? Double-Spending Prevention (闃插弻鑺?
- [ ] 绗?7 绔? 鍗忚缁嗚妭

---

## 馃幆 1. 鏍稿績鐩爣

### 1.1 CryptoNote 瑙ｅ喅鐨勯棶棰?

**Bitcoin 鐨勯殣绉侀棶棰?*:
1. 鉂?**鍙拷韪€?*: 鎵€鏈変氦鏄撳叕寮€鍙
2. 鉂?**鍦板潃鍏宠仈**: 閲嶅浣跨敤鍦板潃娉勯湶韬唤
3. 鉂?**閲戦閫忔槑**: 鎵€鏈夐噾棰濇槑鏂囧彲瑙?

**CryptoNote 鐨勮В鍐虫柟妗?*:
1. 鉁?**涓嶅彲杩借釜**: Ring Signatures 闅愯棌鍙戦€佹柟
2. 鉁?**涓嶅彲鍏宠仈**: One-time Keys 姣忔鐢熸垚鏂板湴鍧€
3. 鉁?**閲戦闅愯棌**: RingCT (v1.0 鍚庡紩鍏? 涓嶅湪鍘熺櫧鐨功)

### 1.2 璁捐鍘熷垯

- **闅愮榛樿**: 涓嶆槸鍙€夌壒鎬? 鑰屾槸寮哄埗闅愮
- **鍘讳腑蹇冨寲**: PoW 鎸栫熆 (CryptoNight 绠楁硶)
- **鍙璁℃€?*: 鎺ユ敹鏂瑰彲閫夋嫨鎬у叕寮€浜ゆ槗 (View Key)

---

## 馃攽 2. One-time Keys (涓€娆℃€у瘑閽?

### 2.1 鏍稿績鎬濇兂

**闂**: Bitcoin 鍦板潃閲嶅浣跨敤瀵艰嚧闅愮娉勯湶

**瑙ｅ喅**: 姣忕瑪浜ゆ槗鐢熸垚鍞竴鐨勪竴娆℃€у湴鍧€

### 2.2 瀵嗛挜浣撶郴

**鎺ユ敹鏂归挶鍖?*:
```
绉侀挜瀵?
- (a, A): Spend key pair (鑺辫垂瀵嗛挜)
  A = aG (鍏挜, 鍏紑)
  
- (b, B): View key pair (瑙嗗浘瀵嗛挜)
  B = bG (鍏挜, 鍏紑)

閽卞寘鍦板潃 = (A, B) 鐨勭紪鐮?
```

**涓轰粈涔堥渶瑕佷袱涓瘑閽ュ?**
- **Spend key (a)**: 绛惧悕浜ゆ槗 (鑺辫垂鏉冮檺)
- **View key (b)**: 鎵弿浜ゆ槗 (鍙鏉冮檺)
- 鍒嗙 鈫?鍙互瀹夊叏鎺堟潈绗笁鏂瑰璁?(鍙粰 View Key)

### 2.3 涓€娆℃€у湴鍧€鐢熸垚

**鍙戦€佹柟娴佺▼**:

```
杈撳叆:
- 鎺ユ敹鏂瑰湴鍧€: (A, B)
- 杈撳嚭绱㈠紩: n (绗嚑涓緭鍑?

姝ラ:
1. 闅忔満鐢熸垚 r (浜ゆ槗绉侀挜, ephemeral)
2. 璁＄畻 R = rG (浜ゆ槗鍏挜, 鏀惧湪閾句笂)
3. 璁＄畻鍏变韩绉樺瘑: S = rA (ECDH)
4. 娲剧敓涓€娆℃€х閽ュ搱甯? h = H_s(S, n)
5. 璁＄畻涓€娆℃€у叕閽? P = H_s(S, n)G + B

杈撳嚭:
- 閾句笂鍏紑: (R, P)
- 鎺ユ敹鏂瑰彲瑙? P 瀵瑰簲閲戦
```

**鎺ユ敹鏂硅瘑鍒?*:

```
杈撳叆:
- 閾句笂鏁版嵁: (R, P)
- 鑷繁鐨勭閽? (a, b)

姝ラ:
1. 璁＄畻鍏变韩绉樺瘑: S = aR (= arG = rA, ECDH)
2. 娲剧敓鍝堝笇: h = H_s(S, n)
3. 璁＄畻鏈熸湜鍏挜: P' = H_s(S, n)G + B
4. 妫€鏌? P' == P?
   - 濡傛灉鐩哥瓑 鈫?杩欑瑪杈撳嚭灞炰簬鎴?
   - 璁板綍 (P, h) 鐢ㄤ簬鍚庣画鑺辫垂

鑺辫垂:
- 涓€娆℃€х閽? p = h + b (鎺ㄥ鍑烘潵鐨?
- 楠岃瘉: pG = hG + bG = hG + B = P 鉁?
```

### 2.4 鏁板璇佹槑

**姝ｇ‘鎬ц瘉鏄?*:

```
鍙戦€佹柟: P = H_s(rA, n)G + B
鎺ユ敹鏂? p = H_s(aR, n) + b

楠岃瘉 pG = P:
  pG = (H_s(aR, n) + b)G
     = H_s(aR, n)G + bG
     = H_s(arG, n)G + B    (鍥犱负 aR = arG = rA)
     = H_s(rA, n)G + B
     = P 鉁?
```

**瀹夊叏鎬?*:
- 澶栭儴瑙傚療鑰呭彧鑳界湅鍒伴殢鏈哄叕閽?P (鏃犳硶鍏宠仈鎺ユ敹鏂?
- 鍙湁鐭ラ亾 a 鐨勪汉鎵嶈兘璁＄畻 S = aR

---

## 馃幁 3. Ring Signatures (鐜鍚?

### 3.1 鏍稿績姒傚康

**鐩爣**: 闅愯棌浜ゆ槗鐨勭湡瀹炶緭鍏?

**鏂规硶**: 灏嗙湡瀹炶緭鍏ユ贩鍏ュ涓?璇遍サ"杈撳叆 (decoys)

### 3.2 鐜鍚嶅畾涔?

**绛惧悕鑰?*:
- 鎷ユ湁绉侀挜 x_s (鐪熷疄杈撳叆)
- 鍏挜 P_s = x_s G

**鐜垚鍛?*:
- 鍏挜闆嗗悎: {P_0, P_1, ..., P_n}
- 鍏朵腑 P_s 鏄湡瀹炲叕閽? 鍏朵粬鏄楗?

**绛惧悕**:
- 璇佹槑: "鎴戠煡閬撶幆涓煇涓€涓閽?
- 浣嗕笉閫忛湶鏄摢涓€涓?

### 3.3 鐧界毊涔︿腑鐨勭幆绛惧悕鏋勯€?(绠€鍖栫増)

**Traceable Ring Signature** (鍙拷韪幆绛惧悕):

```
鍏紑鍙傛暟:
- 鐜垚鍛? {P_0, P_1, ..., P_n}
- 娑堟伅: m

绛惧悕鑰?(鐭ラ亾 x_s):
1. 璁＄畻 Key Image: I = x_s * H_p(P_s)
   (闃插弻鑺辨爣璁?

2. 閫夋嫨闅忔満鏁? 伪
   璁＄畻: L = 伪G, R = 伪H_p(P_s)

3. 瀵规瘡涓潪绉樺瘑绱㈠紩 i != s:
   - 闅忔満閫夋嫨 q_i, w_i
   - 璁＄畻 L_i = q_i G + w_i P_i
   - 璁＄畻 R_i = q_i H_p(P_i) + w_i I

4. 璁＄畻鎸戞垬: c = H(m, L_0, ..., L_n, R_0, ..., R_n)

5. 鐜舰姹傝В:
   - c_s = c - 危(w_i) mod l
   - q_s = 伪 - c_s * x_s mod l

杈撳嚭绛惧悕:
蟽 = (I, c_0, q_0, ..., c_n, q_n)
```

**楠岃瘉**:

```
杈撳叆: 绛惧悕 蟽, 鐜?{P_0, ..., P_n}, 娑堟伅 m

姝ラ:
1. 閲嶆柊璁＄畻 L_i, R_i:
   L_i = q_i G + c_i P_i
   R_i = q_i H_p(P_i) + c_i I

2. 楠岃瘉鎸戞垬:
   H(m, L_0, ..., L_n, R_0, ..., R_n) == 危(c_i) mod l

3. 鎺ュ彈 iff 楠岃瘉閫氳繃
```

### 3.4 涓?Monero CLSAG 鐨勫尯鍒?

**鐧界毊涔︾鍚?* (2013):
- 鍩虹鍙拷韪幆绛惧悕
- 绛惧悕澶у皬: O(n) (n = ring size)
- 姣忎釜鐜垚鍛橀渶瑕?2 涓爣閲?(q_i, c_i)

**Monero CLSAG** (2020):
- 浼樺寲鐨勭畝娲佺幆绛惧悕
- 绛惧悕澶у皬: O(n) 浣嗗父鏁版洿灏?
- 楠岃瘉閫熷害鏇村揩 (~2x)

**SuperVM 搴旈€夋嫨**: CLSAG (鏇寸幇浠? Monero 宸查獙璇?

---

## 馃毇 4. Double-Spending Prevention (闃插弻鑺?

### 4.1 Key Image 鏈哄埗

**闂**: 鐜鍚嶉殣钘忕湡瀹炶緭鍏? 濡備綍闃叉閲嶅鑺辫垂?

**瑙ｅ喅**: Key Image 浣滀负"鏀エ缂栧彿"

### 4.2 Key Image 鐢熸垚

```
杈撳叆:
- 涓€娆℃€х閽? p (灞炰簬鏌愪釜杈撳嚭 P)
- 涓€娆℃€у叕閽? P = pG

璁＄畻:
I = p * H_p(P)

鎬ц川:
1. 纭畾鎬? 鍚屾牱鐨?p 鐢熸垚鍚屾牱鐨?I
2. 鍞竴鎬? 涓嶅悓鐨?p 鐢熸垚涓嶅悓鐨?I (姒傜巼涓?
3. 涓嶅彲浼€? 涓嶇煡閬?p 鏃犳硶璁＄畻鏈夋晥鐨?I
4. 鏃犲叧鑱旀€? 浠?(P, I) 鏃犳硶鎺ㄥ嚭 p
```

### 4.3 鍙岃姳妫€娴?

**楠岃瘉鑺傜偣缁存姢 Key Image 闆嗗悎**:

```
鍏ㄥ眬鐘舵€? key_image_set = {}

楠岃瘉浜ゆ槗:
1. 鎻愬彇绛惧悕涓殑 Key Image: I
2. 妫€鏌? I 鈭?key_image_set?
   - 鏄?鈫?鎷掔粷 (鍙岃姳!)
   - 鍚?鈫?缁х画楠岃瘉

3. 楠岃瘉鐜鍚嶆湁鏁堟€?
   - 楠岃瘉 I 纭疄鐢辩幆涓煇涓閽ョ敓鎴?
   
4. 鎺ュ彈浜ゆ槗:
   - 娣诲姞 I 鍒?key_image_set
```

### 4.4 瀹夊叏鎬у垎鏋?

**鏀诲嚮鍦烘櫙 1: 閲嶅鑺辫垂鍚屼竴杈撳嚭**
- 闃插尽: Key Image 鐩稿悓 鈫?琚嫆缁?鉁?

**鏀诲嚮鍦烘櫙 2: 浼€?Key Image**
- 闃插尽: 鐜鍚嶉獙璇佸け璐?(鏃犳硶璇佹槑鐭ラ亾绉侀挜) 鉁?

**鏀诲嚮鍦烘櫙 3: 鐢ㄤ笉鍚?Key Image 鑺辫垂鍚屼竴杈撳嚭**
- 闃插尽: 鏁板涓婁笉鍙 (I = p * H_p(P) 鏄‘瀹氭€х殑) 鉁?

---

## 馃攼 5. 鍗忚缁嗚妭

### 5.1 浜ゆ槗缁撴瀯

```
Transaction:
鈹溾攢鈹€ version
鈹溾攢鈹€ unlock_time
鈹溾攢鈹€ inputs: [TxIn]
鈹?  鈹斺攢鈹€ TxIn:
鈹?      鈹溾攢鈹€ ring: [PublicKey]  (鐜垚鍛?
鈹?      鈹溾攢鈹€ key_offsets        (瀛樺偍浼樺寲)
鈹?      鈹斺攢鈹€ signature          (鐜鍚?+ Key Image)
鈹斺攢鈹€ outputs: [TxOut]
    鈹斺攢鈹€ TxOut:
        鈹溾攢鈹€ amount             (閲戦, RingCT 鍓嶆槑鏂?
        鈹斺攢鈹€ target: PublicKey  (涓€娆℃€у叕閽?P)
```

### 5.2 鍦板潃缂栫爜

**鏍囧噯鍦板潃**: Base58(network_byte + A + B + checksum)
- network_byte: 涓荤綉/娴嬭瘯缃戞爣璇?
- A: Spend public key (32 bytes)
- B: View public key (32 bytes)
- checksum: 鍓?4 瀛楄妭鍝堝笇

### 5.3 鎸栫熆涓?PoW

**CryptoNight 绠楁硶** (鐧界毊涔︽彁鍑?:
- ASIC 鎶楁€? 闇€瑕?2 MB 鍐呭瓨
- CPU 鍙嬪ソ: 鏅€氱數鑴戝彲鎸栫熆
- Monero 鍚庢潵鏀圭敤 RandomX

---

## 馃搳 6. 鎬ц兘涓庢潈琛?

### 6.1 浜ゆ槗澶у皬

**瀵规瘮 Bitcoin**:

| 鎸囨爣 | Bitcoin | CryptoNote (ring=11) |
|------|---------|---------------------|
| 鍗曡緭鍏ュぇ灏?| ~150 bytes | ~1.5 KB (10x) |
| 鍗曡緭鍑哄ぇ灏?| ~40 bytes | ~100 bytes (2.5x) |
| 鍏稿瀷浜ゆ槗 | ~250 bytes | ~3 KB |

**鏉冭　**: 闅愮 鈫?瀛樺偍/甯﹀

### 6.2 楠岃瘉鏃堕棿

**鐜鍚嶉獙璇?*: O(n) 鏃堕棿澶嶆潅搴?
- ring_size = 11 鈫?~5ms
- ring_size = 64 鈫?~30ms

**鎵归噺楠岃瘉浼樺寲**: 鍙苟琛岄獙璇佸涓鍚?

---

## 馃幆 7. 搴旂敤鍒?SuperVM

### 7.1 鏍稿績璁捐鍐崇瓥

**浠庣櫧鐨功瀛﹀埌鐨勭粡楠?*:

1. **瀵嗛挜浣撶郴**: 閲囩敤鍙屽瘑閽?(Spend + View)
   - 鉁?鍏佽鍙瀹¤ (View Key)
   - 鉁?鍐烽挶鍖呭弸濂?(Spend Key 绂荤嚎)

2. **涓€娆℃€у湴鍧€**: 蹇呴』瀹炵幇
   - 鉁?闃叉鍦板潃鍏宠仈
   - 瀹炵幇: `stealth_address.rs`

3. **鐜鍚?*: 閫夋嫨 CLSAG (涓嶆槸鐧界毊涔﹀師鐗?
   - 鍘熷洜: Monero 7 骞村疄鎴橀獙璇?
   - 瀹炵幇: `ring_signature.rs`

4. **Key Image**: 蹇呴』瀹炵幇
   - 鉁?闃插弻鑺辨牳蹇冩満鍒?
   - 瀛樺偍: 闇€瑕侀珮鏁堢储寮?(Week 7-8 璁捐)

### 7.2 API 璁捐鑽夋

```rust
// 鍩轰簬 CryptoNote 鐞嗚璁捐

pub struct WalletKeys {
    pub spend_secret: SecretKey,   // a
    pub spend_public: PublicKey,   // A = aG
    pub view_secret: SecretKey,    // b
    pub view_public: PublicKey,    // B = bG
}

pub fn generate_one_time_address(
    recipient_spend_public: &PublicKey,
    recipient_view_public: &PublicKey,
    tx_secret_key: &SecretKey,
    output_index: u32,
) -> (PublicKey, PublicKey) {
    // 杩斿洖: (R, P)
    // R = rG (浜ゆ槗鍏挜)
    // P = H_s(rA, n)G + B (涓€娆℃€у叕閽?
    todo!()
}

pub fn scan_transaction(
    tx_public_key: &PublicKey,      // R
    output_public_key: &PublicKey,  // P
    view_secret_key: &SecretKey,    // b
    spend_public_key: &PublicKey,   // B
    output_index: u32,
) -> Option<SecretKey> {
    // 濡傛灉灞炰簬鑷繁, 杩斿洖涓€娆℃€х閽?p
    todo!()
}
```

### 7.3 瀹炵幇浼樺厛绾?

**Week 9-12** (Phase 2.2.1):
- [x] 鐞嗚瀛︿範 鉁?(鏈瑪璁?
- [ ] 瀹炵幇 Key Image 鐢熸垚
- [ ] 瀹炵幇 CLSAG 绛惧悕/楠岃瘉

**Week 13-16** (Phase 2.2.2):
- [ ] 瀹炵幇涓€娆℃€у湴鍧€鐢熸垚
- [ ] 瀹炵幇浜ゆ槗鎵弿
- [ ] 閽卞寘闆嗘垚

---

## 馃摎 8. 寤朵几闃呰

### 8.1 蹇呰璧勬簮

- [x] **CryptoNote v1.0 Whitepaper** (鏈瑪璁?
- [ ] **Zero to Monero 2.0** - 鏇磋缁嗙殑鏁板鎺ㄥ
  - https://web.getmonero.org/library/Zero-to-Monero-2-0-0.pdf
  - 绔犺妭 3-5 閲嶇偣闃呰

- [ ] **MRL 璁烘枃** (Monero Research Lab)
  - MRL-0005: Ring CT 2.0
  - MRL-0011: CLSAG

### 8.2 鐩稿叧鎶€鏈?

- **Bulletproofs** - 鑼冨洿璇佹槑 (涓嶅湪鍘熺櫧鐨功)
- **Stealth Addresses** - 鏇磋缁嗙殑瀹炵幇
- **Subaddresses** - Monero 鎵╁睍 (鏂逛究澶氬湴鍧€绠＄悊)

### 8.3 瀵规瘮鐮旂┒

| 椤圭洰 | 鐜鍚?| 闅愯韩鍦板潃 | 閲戦闅愯棌 | zkSNARK |
|------|-------|---------|---------|---------|
| CryptoNote | 鉁?| 鉁?| 鉂?| 鉂?|
| Monero | 鉁?(CLSAG) | 鉁?| 鉁?(RingCT) | 鉂?|
| Zcash | 鉂?| 鉂?| 鉁?| 鉁?|
| SuperVM | 鉁?(璁″垝) | 鉁?(璁″垝) | 鉁?(璁″垝) | ? (Week 3-4 璇勪及) |

---

## 鉁?瀛︿範杩涘害

### Week 1 (2025-11-04 鑷?2025-11-10)

**Day 1 (2025-11-04)**:
- [x] 涓嬭浇鐧界毊涔?
- [x] 鍒涘缓瀛︿範绗旇妗嗘灦
- [ ] 闃呰绗?1-3 绔?(鑳屾櫙 + 涓嶅彲杩借釜鎬?

**Day 2**:
- [ ] 闃呰绗?4 绔?(One-time Keys) - 璇︾粏鎺ㄥ
- [ ] 鎵嬪姩楠岃瘉鏁板鍏紡

**Day 3**:
- [ ] 闃呰绗?5 绔?(Ring Signatures) - 绛惧悕鏋勯€?
- [ ] 瀵规瘮 Monero CLSAG 鏀硅繘

**Day 4-5**:
- [ ] 闃呰绗?6 绔?(Double-Spending) - Key Image
- [ ] 闃呰绗?7 绔?(鍗忚缁嗚妭)
- [ ] 瀹屾垚鐧界毊涔﹀叏鏂?

**Day 6-7**:
- [ ] 缁撳悎 Monero 婧愮爜楠岃瘉鐞嗚
- [ ] 鎬荤粨鏍稿績姒傚康
- [ ] 鍑嗗 Week 2 瀹炶返

### Week 2 (2025-11-11 鑷?2025-11-17)

**Day 8-10**:
- [ ] 闃呰 Zero to Monero Ch3-5
- [ ] 娣卞叆 RingCT 鏁板鎺ㄥ

**Day 11-14**:
- [ ] 缂栧啓鎶€鏈€夊瀷鎶ュ憡
- [ ] 璁捐 SuperVM 闅愮 API
- [ ] 鍑嗗 Week 3-4 zkSNARK 璇勪及

---

## 馃挕 闂涓庢€濊€?

### 鍏抽敭闂

1. **涓轰粈涔堥渶瑕?H_p (Hash-to-Point)?**
   - 绛? 纭繚 Key Image 鍦ㄦ洸绾夸笂, 涓旀棤娉曢娴?

2. **濡備綍閫夋嫨 Ring Members?**
   - 鐧界毊涔? 闅忔満閫夋嫨
   - Monero: Gamma 鍒嗗竷 (妯℃嫙鐪熷疄鑺辫垂妯″紡)

3. **View Key 娉勯湶鐨勯闄?**
   - 鍙互鐪嬪埌鎵€鏈変氦鏄撻噾棰濆拰鎺ユ敹璁板綍
   - 浣嗘棤娉曡姳璐?(闇€瑕?Spend Key)
   - 鐢ㄩ€? 瀹¤, 浜ゆ槗鎵€鍚堣

### 涓汉鐞嗚В

**CryptoNote 鐨勫ぉ鎵嶈璁?*:
1. 鐜鍚?鈫?闅愯棌鍙戦€佹柟
2. 涓€娆℃€у湴鍧€ 鈫?闅愯棌鎺ユ敹鏂?
3. Key Image 鈫?闃插弻鑺?(涓嶆硠闇茶韩浠?

涓夎€呯粨鍚?= 瀹屾暣闅愮灞?

**TODO**: 姣忔棩璁板綍鏂扮殑鐞嗚В

---

## 馃敆 鐩稿叧绗旇

- `monero-study-notes.md` - Monero 婧愮爜瀹炵幇
- `curve25519-dalek-notes.md` - 瀵嗙爜瀛﹀簱瀛︿範
- `ring-signature-deep-dive.md` (Week 2 鍒涘缓)

---

**鏈€鍚庢洿鏂?*: 2025-11-04  
**涓嬫鏇存柊**: 姣忔棩鍚屾瀛︿範杩涘害




