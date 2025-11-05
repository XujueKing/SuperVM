# Monero 婧愮爜瀛︿範绗旇

**鐮旂┒鍛ㄦ湡**: Week 1-2 (2025-11-04 鑷?2025-11-17)  
**浠撳簱璺緞**: `d:\WEB3_AI寮€鍙慭monero-research`  
**瀛︿範鐩爣**: 鐞嗚В Ring Signature, Stealth Address, Key Image 瀹炵幇缁嗚妭

---

## 馃搵 瀛︿範娓呭崟

- [ ] Ring Signature 瀹炵幇
- [ ] Stealth Address 鐢熸垚鏈哄埗
- [ ] Key Image 闃插弻鑺?
- [ ] RingCT 瀹屾暣浜ゆ槗娴佺▼

---

## 馃攳 1. Ring Signature (鐜鍚?

### 1.1 鏍稿績鏂囦欢瀹氫綅

**鍏抽敭鏂囦欢**:
- `src/ringct/rctSigs.cpp` - RingCT 绛惧悕瀹炵幇
- `src/ringct/rctTypes.h` - RingCT 绫诲瀷瀹氫箟
- `src/cryptonote_core/cryptonote_tx_utils.cpp` - 浜ゆ槗鏋勯€?
- `src/crypto/crypto.cpp` - 鍩虹鍔犲瘑鍘熻

### 1.2 Ring Signature 绫诲瀷

Monero 鏀寔澶氱鐜鍚嶇畻娉?

| 绠楁硶 | 寮曞叆鐗堟湰 | 绛惧悕澶у皬 | 楠岃瘉閫熷害 | 鐗圭偣 |
|------|---------|---------|---------|------|
| MLSAG | v7 (2016) | ~1.7 KB (ring=11) | 鎱?| 澶氬眰閾炬帴鍖垮悕缇ょ鍚?|
| CLSAG | v12 (2020) | ~1.5 KB (ring=11) | 蹇?| 绠€娲侀摼鎺ュ尶鍚嶇兢绛惧悕 |
| Triptych | 鎻愭涓?| ~1.2 KB | 鏇村揩 | 鍩轰簬瀵规暟澶у皬璇佹槑 |

**褰撳墠瀹炵幇**: CLSAG (鑷?v12 Monero 鍗忚)

### 1.3 CLSAG 绛惧悕缁撴瀯

```cpp
// src/ringct/rctTypes.h (Monero)
struct clsag {
    rct::keyV s; // scalars (responses), length = ring_size
    rct::key c1; // initial challenge
    rct::key I;  // signing key image (prevents double-spend)
    rct::key D;  // commitment key image (binds to commitment relation)
};
```

瀛楁瑙ｉ噴:
- s: 鍝嶅簲鏍囬噺鍚戦噺锛岄暱搴︾瓑浜庣幆澶у皬 N銆傚搴旀瘡涓幆鎴愬憳鐨勫搷搴斿€硷紝鐢ㄤ簬闂悎鎸戞垬鐜€?
- c1: 绗竴涓寫鎴樻爣閲忥紝浣滀负鏁翠釜 Fiat-Shamir 鎸戞垬閾剧殑璧风偣銆?
- I: 瀵嗛挜闀滃儚 (Key Image)锛岀敱鐪熷疄绉侀挜 x 鍜屽叾鍏挜 P 缁?Hp(P) 鐢熸垚锛岀‘淇濆悓涓€杈撳嚭琚姳璐规椂鍙娴嬮噸澶嶏紝涓斾笉鏆撮湶鐪熷疄杈撳叆銆?
- D: 鎵胯闀滃儚 (Commitment Key Image)锛屽皢鎵胯鍏崇郴缁戝畾杩涚鍚嶏紝鎶靛尽鈥淛anus/缁勫悎鈥濈被鏀诲嚮锛岀‘淇濈鍚嶅悓鏃堕摼鎺ュ埌鎵胯鐨勭洸鍥犲瓙鍏崇郴銆?

娉ㄦ剰:
- I 鍦?prunable 搴忓垪鍖栦腑涓嶄繚瀛橈紝鍙緷鎹緭鍏ヤ笌鐜垚鍛橀噸寤猴紱D 浼氳搴忓垪鍖栦互渚涢獙璇併€?

### 1.4 CLSAG 绛惧悕绠楁硶娴佺▼

#### 绛惧悕鐢熸垚 (`CLSAG_Gen` + `proveRctCLSAGSimple`)

**鍑芥暟瀹氫綅**: `src/ringct/rctSigs.cpp`
- 鏍稿績鍑芥暟: `CLSAG_Gen()` (L1100-1300)
- 绠€鍖栨帴鍙? `proveRctCLSAGSimple()` (L1800+)

**绠楁硶姝ラ**:

```cpp
// 杈撳叆:
// - message: 浜ゆ槗娑堟伅鍝堝笇
// - P: 鐜垚鍛樺叕閽ュ悜閲?[P_0, P_1, ..., P_n]
// - p: 鐪熷疄绉侀挜 (瀵瑰簲 P[l])
// - C: 鎵胯鍚戦噺 [C_0, C_1, ..., C_n] (宸插噺鍘?C_offset)
// - z: 鎵胯鐩插洜瀛?
// - C_nonzero: 鍘熷鎵胯鍚戦噺 (鐢ㄤ簬鍝堝笇)
// - C_offset: 鎵胯鍋忕Щ閲?(閫氬父鏄緭鍑烘壙璇?
// - l: 鐪熷疄瀵嗛挜绱㈠紩 (secret index)

clsag CLSAG_Gen(...) {
    size_t n = P.size(); // 鐜ぇ灏?
    
    // 姝ラ 1: 鐢熸垚瀵嗛挜闀滃儚
    ge_p3 H_p3;
    hash_to_p3(H_p3, P[l]);              // H = Hp(P[l])
    key H;
    ge_p3_tobytes(H.bytes, &H_p3);
    
    key D;                                // 鎵胯瀵嗛挜闀滃儚
    key I;                                // 绛惧悕瀵嗛挜闀滃儚
    
    // 姝ラ 2: 鍒濆鍖栭殢鏈哄€?(鐢辩‖浠惰澶囨垨杞欢鐢熸垚)
    key a, aG, aH;
    hwdev.clsag_prepare(p, z, I, D, H, a, aG, aH);
    // 鍏朵腑: I = p * H, D = z * H, aG = a*G, aH = a*H
    
    // 姝ラ 3: 棰勮绠楀瘑閽ラ暅鍍?
    geDsmp I_precomp, D_precomp;
    precomp(I_precomp.k, I);
    precomp(D_precomp.k, D);
    
    sig.D = scalarmult8(D);               // D' = 8*D (cofactor 娓呴櫎)
    
    // 姝ラ 4: 璁＄畻鑱氬悎鍝堝笇 mu_P, mu_C (鍩熷垎绂?
    keyV mu_P_to_hash(2*n+4);
    keyV mu_C_to_hash(2*n+4);
    
    // 鍩熷垎绂绘爣绛?
    sc_0(mu_P_to_hash[0].bytes);
    memcpy(mu_P_to_hash[0].bytes, config::HASH_KEY_CLSAG_AGG_0, ...);
    sc_0(mu_C_to_hash[0].bytes);
    memcpy(mu_C_to_hash[0].bytes, config::HASH_KEY_CLSAG_AGG_1, ...);
    
    // 濉厖鍏挜鍜屾壙璇?
    for (size_t i = 1; i < n+1; ++i) {
        mu_P_to_hash[i] = P[i-1];
        mu_C_to_hash[i] = P[i-1];
        mu_P_to_hash[i+n] = C_nonzero[i-n-1];
        mu_C_to_hash[i+n] = C_nonzero[i-n-1];
    }
    mu_P_to_hash[2*n+1] = I;
    mu_P_to_hash[2*n+2] = sig.D;
    mu_P_to_hash[2*n+3] = C_offset;
    // mu_C_to_hash 鍚岀悊...
    
    key mu_P = hash_to_scalar(mu_P_to_hash);
    key mu_C = hash_to_scalar(mu_C_to_hash);
    
    // 姝ラ 5: 璁＄畻鍒濆鎸戞垬 c[l+1]
    keyV c_to_hash(2*n+5);  // domain, P, C, C_offset, message, L, R
    sc_0(c_to_hash[0].bytes);
    memcpy(c_to_hash[0].bytes, config::HASH_KEY_CLSAG_ROUND, ...);
    
    for (size_t i = 1; i < n+1; ++i) {
        c_to_hash[i] = P[i-1];
        c_to_hash[i+n] = C_nonzero[i-1];
    }
    c_to_hash[2*n+1] = C_offset;
    c_to_hash[2*n+2] = message;
    c_to_hash[2*n+3] = aG;  // L_initial
    c_to_hash[2*n+4] = aH;  // R_initial
    
    key c;
    hwdev.clsag_hash(c_to_hash, c);
    
    // 姝ラ 6: 鐜舰璁＄畻鎸戞垬鍜屽搷搴?
    sig.s = keyV(n);
    size_t i = (l + 1) % n;
    if (i == 0) copy(sig.c1, c);  // 淇濆瓨 c1
    
    key c_new, L, R, c_p, c_c;
    geDsmp P_precomp, C_precomp, H_precomp;
    ge_p3 Hi_p3;
    
    while (i != l) {
        // 涓洪潪绉樺瘑绱㈠紩鐢熸垚闅忔満鍝嶅簲
        sig.s[i] = skGen();
        
        sc_mul(c_p.bytes, mu_P.bytes, c.bytes);  // c_p = c * mu_P
        sc_mul(c_c.bytes, mu_C.bytes, c.bytes);  // c_c = c * mu_C
        
        // 棰勮绠楃偣
        precomp(P_precomp.k, P[i]);
        precomp(C_precomp.k, C[i]);
        
        // 璁＄畻 L = s[i]*G + c_p*P[i] + c_c*C[i]
        addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
        
        // 璁＄畻 R = s[i]*Hp(P[i]) + c_p*I + c_c*D
        hash_to_p3(Hi_p3, P[i]);
        ge_dsm_precomp(H_precomp.k, &Hi_p3);
        addKeys_aAbBcC(R, sig.s[i], H_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
        
        // 璁＄畻涓嬩竴涓寫鎴?
        c_to_hash[2*n+3] = L;
        c_to_hash[2*n+4] = R;
        hwdev.clsag_hash(c_to_hash, c_new);
        copy(c, c_new);
        
        i = (i + 1) % n;
        if (i == 0) copy(sig.c1, c);  // 淇濆瓨鐜捣鐐?
    }
    
    // 姝ラ 7: 璁＄畻鐪熷疄绱㈠紩鐨勫搷搴?(闂悎鐜?
    // s[l] = a - c*(p*mu_P + z*mu_C) mod l
    hwdev.clsag_sign(c, a, p, z, mu_P, mu_C, sig.s[l]);
    
    // 娓呯悊鏁忔劅鏁版嵁
    memwipe(&a, sizeof(key));
    
    return sig;  // 杩斿洖 (s, c1, I, D)
}
```

#### 绛惧悕楠岃瘉 (`verRctCLSAGSimple`)

**鍑芥暟瀹氫綅**: `src/ringct/rctSigs.cpp:L2900+`

**楠岃瘉姝ラ**:

```cpp
bool verRctCLSAGSimple(const key &message, const clsag &sig, 
                       const ctkeyV &pubs, const key &C_offset) {
    const size_t n = pubs.size();
    
    // 姝ラ 1: 鏁版嵁瀹屾暣鎬ф鏌?
    CHECK(n >= 1);
    CHECK(n == sig.s.size());
    for (size_t i = 0; i < n; ++i)
        CHECK(sc_check(sig.s[i].bytes) == 0);  // 鏍囬噺鍚堟硶鎬?
    CHECK(sc_check(sig.c1.bytes) == 0);
    CHECK(!(sig.I == identity()));  // Key Image 涓嶈兘鏄崟浣嶅厓
    
    // 姝ラ 2: 棰勫鐞嗘壙璇哄亸绉?
    ge_p3 C_offset_p3;
    ge_frombytes_vartime(&C_offset_p3, C_offset.bytes);
    ge_cached C_offset_cached;
    ge_p3_to_cached(&C_offset_cached, &C_offset_p3);
    
    // 姝ラ 3: 棰勮绠楀瘑閽ラ暅鍍?
    key D_8 = scalarmult8(sig.D);
    CHECK(!(D_8 == identity()));
    
    geDsmp I_precomp, D_precomp;
    precomp(I_precomp.k, sig.I);
    precomp(D_precomp.k, D_8);
    
    // 姝ラ 4: 閲嶅缓鑱氬悎鍝堝笇 mu_P, mu_C (涓庣鍚嶆椂鐩稿悓)
    keyV mu_P_to_hash(2*n+4);
    keyV mu_C_to_hash(2*n+4);
    // ... (濉厖閫昏緫鍚岀鍚?
    key mu_P = hash_to_scalar(mu_P_to_hash);
    key mu_C = hash_to_scalar(mu_C_to_hash);
    
    // 姝ラ 5: 璁剧疆杞鍝堝笇
    keyV c_to_hash(2*n+5);
    // ... (濉厖閫昏緫鍚岀鍚?
    c_to_hash[2*n+2] = message;
    
    // 姝ラ 6: 浠?c1 寮€濮嬮噸寤烘寫鎴樼幆
    key c = copy(sig.c1);
    key c_p, c_c, c_new, L, R;
    geDsmp P_precomp, C_precomp, hash_precomp;
    ge_p3 hash8_p3, temp_p3;
    ge_p1p1 temp_p1;
    
    size_t i = 0;
    while (i < n) {
        sc_mul(c_p.bytes, mu_P.bytes, c.bytes);
        sc_mul(c_c.bytes, mu_C.bytes, c.bytes);
        
        // 棰勮绠?
        precomp(P_precomp.k, pubs[i].dest);
        
        // 璁＄畻 C[i] - C_offset
        ge_frombytes_vartime(&temp_p3, pubs[i].mask.bytes);
        ge_sub(&temp_p1, &temp_p3, &C_offset_cached);
        ge_p1p1_to_p3(&temp_p3, &temp_p1);
        ge_dsm_precomp(C_precomp.k, &temp_p3);
        
        // 閲嶅缓 L 鍜?R (楠岃瘉鏂圭▼)
        // L = s[i]*G + c_p*P[i] + c_c*(C[i] - C_offset)
        addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
        
        // R = s[i]*Hp(P[i]) + c_p*I + c_c*D
        hash_to_p3(hash8_p3, pubs[i].dest);
        ge_dsm_precomp(hash_precomp.k, &hash8_p3);
        addKeys_aAbBcC(R, sig.s[i], hash_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
        
        // 璁＄畻涓嬩竴涓寫鎴?
        c_to_hash[2*n+3] = L;
        c_to_hash[2*n+4] = R;
        c_new = hash_to_scalar(c_to_hash);
        CHECK(!(c_new == zero()));
        
        copy(c, c_new);
        i = i + 1;
    }
    
    // 姝ラ 7: 楠岃瘉鐜棴鍚?(c 搴旇鍥炲埌 c1)
    sc_sub(c_new.bytes, c.bytes, sig.c1.bytes);
    return sc_isnonzero(c_new.bytes) == 0;  // c == c1 鍒欓€氳繃
}
```

#### 鍏抽敭鏁板鍏崇郴

**绛惧悕姝ｇ‘鎬ц瘉鏄?*:

瀵逛簬鐪熷疄绱㈠紩 l, 鍝嶅簲 s[l] 鐨勮绠?
```
s[l] = a - c[l]*(p*mu_P + z*mu_C) mod l
```

楠岃瘉鏃堕噸寤?
```
L[l] = s[l]*G + c_p[l]*P[l] + c_c[l]*C[l]
     = (a - c[l]*(p*mu_P + z*mu_C))*G + c[l]*mu_P*P[l] + c[l]*mu_C*C[l]
     = a*G + c[l]*mu_P*(P[l] - p*G) + c[l]*mu_C*(C[l] - z*G)
     = a*G  (鍥犱负 P[l] = p*G, C[l] = z*G)
     = aG (鍒濆鍊?

R[l] = s[l]*Hp(P[l]) + c_p[l]*I + c_c[l]*D
     = ... (绫讳技鎺ㄥ)
     = aH (鍒濆鍊?
```

鍥犳楠岃瘉鏃朵細閲嶅缓鍑?(L[l], R[l]) = (aG, aH), 浠庤€岄噸寤哄嚭 c[l+1], 鏈€缁堥棴鍚堢幆鍥炲埌 c1.

**鍏抽敭闂瑙ｇ瓟**:
- 鉁?**Ring members 閫夋嫨**: 鐢遍挶鍖呴€氳繃 `get_outs` RPC 浠庡尯鍧楅摼鑾峰彇, 浣跨敤 gamma 鍒嗗竷閫夋嫨 decoys
- 鉁?**Key Image 鐢熸垚**: I = p * Hp(P), 鍏朵腑 Hp(P) = hash_to_p3(P) 灏嗗叕閽ュ搱甯屽埌鏇茬嚎鐐?
- 鉁?**楠岃瘉鏈夋晥鎬?*: 閲嶅缓鎸戞垬鐜? 妫€鏌?c_final == c1 (鐜棴鍚?

### 1.5 浠ｇ爜鐗囨鍒嗘瀽

#### 鐗囨 1: 瀵嗛挜闀滃儚鐢熸垚 (Key Image Generation)

```cpp
// src/ringct/rctSigs.cpp:L1120-1130
// 浠庡叕閽ョ敓鎴?Hash-to-Point
ge_p3 H_p3;
hash_to_p3(H_p3, P[l]);  // H = Hp(P[l])
key H;
ge_p3_tobytes(H.bytes, &H_p3);

// 纭欢璁惧璁＄畻 I = p * H, D = z * H
key a, aG, aH;
hwdev.clsag_prepare(p, z, sig.I, D, H, a, aG, aH);
```

**鍘熺悊**: 
- `hash_to_p3(P)` 灏?32 瀛楄妭鍏挜 P 纭畾鎬ф槧灏勫埌妞渾鏇茬嚎鐐?Hp(P)
- Key Image I = x * Hp(P) 缁戝畾鍒扮閽?x, 浣嗕笉鏆撮湶 x
- 鍚屼竴杈撳嚭鍐嶆鑺辫垂浼氫骇鐢熺浉鍚岀殑 I (鍏ㄧ綉鍙娴嬪弻鑺?

#### 鐗囨 2: 鑱氬悎鍝堝笇璁＄畻 (Aggregation Hashes)

```cpp
// src/ringct/rctSigs.cpp:L1150-1180
// 鍩熷垎绂?(闃叉璺ㄥ崗璁?璺ㄤ笂涓嬫枃鏀诲嚮)
keyV mu_P_to_hash(2*n+4);
sc_0(mu_P_to_hash[0].bytes);
memcpy(mu_P_to_hash[0].bytes, 
       config::HASH_KEY_CLSAG_AGG_0,
       sizeof(config::HASH_KEY_CLSAG_AGG_0)-1);

// 缁戝畾鎵€鏈夊叕閽ュ拰鎵胯鍒板搱甯?
for (size_t i = 1; i < n+1; ++i) {
    mu_P_to_hash[i] = P[i-1];
    mu_P_to_hash[i+n] = C_nonzero[i-n-1];
}
mu_P_to_hash[2*n+1] = sig.I;
mu_P_to_hash[2*n+2] = sig.D;
mu_P_to_hash[2*n+3] = C_offset;

key mu_P = hash_to_scalar(mu_P_to_hash);
key mu_C = hash_to_scalar(mu_C_to_hash); // 绫讳技
```

**浣滅敤**:
- mu_P, mu_C 灏嗘墍鏈夌幆鎴愬憳鍜屾壙璇虹粦瀹氬埌绛惧悕
- 闃叉"娣峰悎鐜?鏀诲嚮 (涓嶅悓绛惧悕鐨勭幆鎴愬憳娣锋穯)
- 鍩熷垎绂绘爣绛?`HASH_KEY_CLSAG_AGG_0/1` 闃叉鍝堝笇閲嶇敤

#### 鐗囨 3: 鐜舰鎸戞垬-鍝嶅簲璁＄畻 (Ring Challenge-Response Loop)

```cpp
// src/ringct/rctSigs.cpp:L1220-1260
while (i != l) {
    sig.s[i] = skGen();  // 闈炵瀵嗙储寮? 闅忔満鍝嶅簲
    
    sc_mul(c_p.bytes, mu_P.bytes, c.bytes);
    sc_mul(c_c.bytes, mu_C.bytes, c.bytes);
    
    precomp(P_precomp.k, P[i]);
    precomp(C_precomp.k, C[i]);
    
    // L = s[i]*G + c_p*P[i] + c_c*C[i]
    addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
    
    // R = s[i]*Hp(P[i]) + c_p*I + c_c*D
    hash_to_p3(Hi_p3, P[i]);
    ge_dsm_precomp(H_precomp.k, &Hi_p3);
    addKeys_aAbBcC(R, sig.s[i], H_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
    
    // 璁＄畻涓嬩竴涓寫鎴?
    c_to_hash[2*n+3] = L;
    c_to_hash[2*n+4] = R;
    hwdev.clsag_hash(c_to_hash, c_new);
    
    copy(c, c_new);
    i = (i + 1) % n;
}
```

**Fiat-Shamir 鍙樻崲**:
- 瀵逛簬 decoys (闈炵瀵嗙储寮?, 鍏堥€夐殢鏈?s[i], 鍐嶈绠?L, R
- 鎸戞垬 c 閫氳繃鍝堝笇閾句紶閫? c[i+1] = H(L[i], R[i], ...)
- 鐪熷疄绱㈠紩 l 鐨?s[l] 鍦ㄧ幆闂悎鏃跺弽鎺? s[l] = a - c[l]*(...)

#### 鐗囨 4: 鐪熷疄鍝嶅簲璁＄畻 (Signing Index Response)

```cpp
// src/ringct/rctSigs.cpp:L1270
// 鐜棴鍚? 璁＄畻 s[l] 浣垮緱楠岃瘉鏂圭▼鎴愮珛
hwdev.clsag_sign(c, a, p, z, mu_P, mu_C, sig.s[l]);

// 杞欢瀹炵幇 (hw::device_default::clsag_sign):
// s[l] = a - c*(p*mu_P + z*mu_C) mod l
sc_mul(tmp1, mu_P, p);         // tmp1 = mu_P * p
sc_mul(tmp2, mu_C, z);         // tmp2 = mu_C * z
sc_add(tmp3, tmp1, tmp2);      // tmp3 = mu_P*p + mu_C*z
sc_mul(tmp4, c, tmp3);         // tmp4 = c * (mu_P*p + mu_C*z)
sc_sub(s, a, tmp4);            // s = a - c*(mu_P*p + mu_C*z)
```

**闆剁煡璇嗘€?*:
- s[l] 娣峰悎浜嗛殢鏈烘暟 a 鍜岀瀵?(p, z)
- 楠岃瘉鑰呮棤娉曚粠 s[l] 鍙嶆帹 p 鎴?z
- 鍙兘楠岃瘉 s[l] 婊¤冻绛惧悕鏂圭▼

#### 鐗囨 5: 楠岃瘉鐜棴鍚?(Verification Ring Closure)

```cpp
// src/ringct/rctSigs.cpp:L3100-3150 (verRctCLSAGSimple)
key c = copy(sig.c1);
size_t i = 0;

while (i < n) {
    // 閲嶅缓 L[i] 鍜?R[i]
    sc_mul(c_p.bytes, mu_P.bytes, c.bytes);
    sc_mul(c_c.bytes, mu_C.bytes, c.bytes);
    
    addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
    addKeys_aAbBcC(R, sig.s[i], hash_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
    
    // 閲嶅缓涓嬩竴涓寫鎴?
    c_to_hash[2*n+3] = L;
    c_to_hash[2*n+4] = R;
    c_new = hash_to_scalar(c_to_hash);
    
    copy(c, c_new);
    i++;
}

// 楠岃瘉鐜棴鍚? c 搴旇鍥炲埌 c1
sc_sub(c_new.bytes, c.bytes, sig.c1.bytes);
return sc_isnonzero(c_new.bytes) == 0;  // c == c1?
```

**楠岃瘉閫昏緫**:
- 浠?c1 寮€濮? 渚濇閲嶅缓鎵€鏈?(L[i], R[i])
- 姣忎釜 L[i], R[i] 鐢熸垚涓嬩竴涓寫鎴?c[i+1]
- 濡傛灉绛惧悕鏈夋晥, 鏈€鍚庣殑 c 浼氱瓑浜庤捣鐐?c1 (鐜棴鍚?
- 鍚﹀垯 c != c1, 绛惧悕鏃犳晥

#### 鎬ц兘浼樺寲鎶€宸?

```cpp
// 1. 棰勮绠?(Precomputation)
geDsmp P_precomp;
precomp(P_precomp.k, P[i]);  // 灏嗙偣杞崲涓?DSM 褰㈠紡, 鍔犻€熷鏍囬噺涔樻硶

// 2. 鎵归噺鎿嶄綔 (Batch Operations)
addKeys_aGbBcC(L, s, c_p, P_precomp.k, c_c, C_precomp.k);
// 绛変环浜? L = s*G + c_p*P + c_c*C (涓€娆¤绠?

// 3. Cofactor 娓呴櫎
key D_8 = scalarmult8(sig.D);  // D' = 8*D, 纭繚鍦ㄧ礌鏁伴樁瀛愮兢
CHECK(!(D_8 == identity()));    // 鎷掔粷灏忓瓙缇ゆ敾鍑?
```

**瀛︿範绗旇**:
- CLSAG 浣跨敤 Fiat-Shamir 鍙樻崲灏嗕氦浜掑紡鍗忚杞负闈炰氦浜掑紡
- 鑱氬悎鍝堝笇 (mu_P, mu_C) 鏄?CLSAG 鐩告瘮 MLSAG 鐨勫叧閿敼杩?
- 鎵胯瀵嗛挜闀滃儚 D 闃叉"Janus/缁勫悎"绫绘敾鍑?
- 鍩熷垎绂绘爣绛鹃槻姝㈣法鍗忚鏀诲嚮 (濡?Monero vs SuperVM 绛惧悕娣锋穯) 

---

## 馃幁 2. Stealth Address (闅愯韩鍦板潃)

### 2.1 鏍稿績姒傚康

**闅愯韩鍦板潃 = 姣忕瑪浜ゆ槗鐢熸垚鍞竴鐨勪竴娆℃€у湴鍧€**

- **鍙戦€佹柟**: 浣跨敤鎺ユ敹鏂瑰叕閽ョ敓鎴愪竴娆℃€у湴鍧€
- **鎺ユ敹鏂?*: 浣跨敤绉侀挜鎵弿鍖哄潡閾捐瘑鍒睘浜庤嚜宸辩殑浜ゆ槗

### 2.2 鍏抽敭鏂囦欢

- `src/cryptonote_basic/cryptonote_format_utils.cpp`
  - `generate_key_derivation()` - 娲剧敓瀵嗛挜
  - `derive_public_key()` - 娲剧敓鍏挜
- `src/wallet/wallet2.cpp`
  - 閽卞寘鎵弿閫昏緫

### 2.3 鍦板潃鐢熸垚娴佺▼

```
鎺ユ敹鏂归挶鍖呭瘑閽?
- Spend key pair: (a, A = aG)  - 鑺辫垂瀵嗛挜
- View key pair: (b, B = bG)   - 瑙嗗浘瀵嗛挜

鍙戦€佹柟鐢熸垚涓€娆℃€у湴鍧€:
1. 闅忔満鐢熸垚 r (浜ゆ槗瀵嗛挜)
2. 璁＄畻 R = rG (浜ゆ槗鍏挜)
3. 璁＄畻鍏变韩绉樺瘑: S = rA = raG
4. 璁＄畻涓€娆℃€у叕閽? P = H(S, n)G + B
   鍏朵腑 n 鏄緭鍑虹储寮?

鎺ユ敹鏂硅瘑鍒?
1. 鎵弿鍖哄潡閾句腑鐨?R
2. 璁＄畻鍏变韩绉樺瘑: S = aR = arG
3. 璁＄畻鏈熸湜鍏挜: P' = H(S, n)G + B
4. 濡傛灉 P' == P, 鍒欒杈撳嚭灞炰簬鑷繁
```

### 2.4 浠ｇ爜鐗囨

**TODO**: 鎻愬彇 `generate_key_derivation()` 瀹炵幇

```cpp
// src/cryptonote_basic/cryptonote_format_utils.cpp

```

**鍏抽敭闂**:
- [ ] 濡備綍楂樻晥鎵弿澶ч噺浜ゆ槗?
- [ ] 瑙嗗浘瀵嗛挜 vs 鑺辫垂瀵嗛挜鐨勫垎绂讳綔鐢?
- [ ] 澶氫釜杈撳嚭濡備綍绱㈠紩 (n)?

---

## 馃攽 2. Stealth Address (闅愯韩鍦板潃)

### 2.1 鍩烘湰姒傚康

**鐩殑**: 姣忕瑪浜ゆ槗鐢熸垚鍞竴鐨勪竴娆℃€у湴鍧€, 闃叉浜ゆ槗鍏宠仈, 淇濇姢鎺ユ敹鏂归殣绉併€?

**鏍稿績鎬濇兂**: 
- 鍙戦€佹柟浣跨敤鎺ユ敹鏂圭殑鍏挜 + 闅忔満鏁扮敓鎴愪竴娆℃€у湴鍧€
- 鎺ユ敹鏂归€氳繃鎵弿鍖哄潡閾? 浣跨敤绉侀挜鎭㈠灞炰簬鑷繁鐨勮緭鍑?
- 鍖哄潡閾句笂姣忎釜杈撳嚭鍦板潃閮戒笉鍚? 浣嗘帴鏀舵柟鍙互鑺辫垂

### 2.2 Monero 鍦板潃缁撴瀯

Monero 鏍囧噯鍦板潃鍖呭惈涓ゅ瀵嗛挜:

```
Address = (A, B)
  A = a*G  (View Public Key, 瑙嗗浘鍏挜)
  B = b*G  (Spend Public Key, 鑺辫垂鍏挜)

鐢ㄦ埛鎸佹湁:
  a = view secret key (瑙嗗浘绉侀挜, 鐢ㄤ簬鎵弿)
  b = spend secret key (鑺辫垂绉侀挜, 鐢ㄤ簬绛惧悕)
```

**鑱岃矗鍒嗙**:
- `a` (view key): 鍙兘鏌ョ湅浜ゆ槗, 涓嶈兘鑺辫垂 (鍙垎浜粰瀹¤鍛?
- `b` (spend key): 鑺辫垂璧勯噾 (缁濆淇濆瘑)

### 2.3 涓€娆℃€у湴鍧€鐢熸垚 (鍙戦€佹柟)

鍙戦€佹柟瑕佺粰鍦板潃 `(A, B)` 鍙戦€佽祫閲戞椂:

#### 姝ラ 1: 鐢熸垚浜ゆ槗瀵嗛挜瀵?

```cpp
// src/crypto/crypto.cpp:L150
secret_key r;  // 浜ゆ槗绉侀挜 (transaction secret key)
random_scalar(r);  // 鐢熸垚闅忔満 256-bit 鏍囬噺

public_key R;  // 浜ゆ槗鍏挜 (transaction public key)
secret_key_to_public_key(r, R);  // R = r*G
```

`R` 浼氳鍐欏叆浜ゆ槗鐨?`tx_extra` 瀛楁, 鍏紑鍙銆?

#### 姝ラ 2: 璁＄畻鍏变韩瀵嗛挜 (Diffie-Hellman)

```cpp
// src/crypto/crypto.cpp:L237 - generate_key_derivation()
bool generate_key_derivation(const public_key &A,     // 鎺ユ敹鏂硅鍥惧叕閽?
                              const secret_key &r,     // 浜ゆ槗绉侀挜
                              key_derivation &derivation) {
    ge_p3 point;
    ge_p2 point2;
    ge_p1p1 point3;
    
    if (ge_frombytes_vartime(&point, &A) != 0)
        return false;
    
    // 璁＄畻 r*A (Diffie-Hellman 瀵嗛挜鍗忓晢)
    ge_scalarmult(&point2, &r, &point);
    
    // 涔樹互 cofactor 8 (闃叉灏忓瓙缇ゆ敾鍑?
    ge_mul8(&point3, &point2);
    ge_p1p1_to_p2(&point2, &point3);
    ge_tobytes(&derivation, &point2);
    
    return true;
}
```

**鏁板鍘熺悊** (ECDH):
```
derivation = 8 * r * A
           = 8 * r * (a*G)
           = 8 * a * (r*G)
           = 8 * a * R
```

鍙戦€佹柟浣跨敤 `(r, A)` 璁＄畻, 鎺ユ敹鏂逛娇鐢?`(a, R)` 璁＄畻, 缁撴灉鐩稿悓!

#### 姝ラ 3: 娲剧敓涓€娆℃€ц緭鍑哄叕閽?

```cpp
// src/crypto/crypto.cpp:L258 - derive_public_key()
bool derive_public_key(const key_derivation &derivation,
                       size_t output_index,     // 杈撳嚭绱㈠紩 n
                       const public_key &B,     // 鎺ユ敹鏂硅姳璐瑰叕閽?
                       public_key &P_out) {     // 涓€娆℃€у叕閽?
    ec_scalar scalar;
    ge_p3 point1, point2;
    ge_cached point3;
    ge_p1p1 point4;
    ge_p2 point5;
    
    if (ge_frombytes_vartime(&point1, &B) != 0)
        return false;
    
    // 璁＄畻 Hs(derivation || output_index)
    derivation_to_scalar(derivation, output_index, scalar);
    
    // 璁＄畻 Hs(...)*G
    ge_scalarmult_base(&point2, &scalar);
    
    // P_out = Hs(...)*G + B
    ge_p3_to_cached(&point3, &point2);
    ge_add(&point4, &point1, &point3);
    ge_p1p1_to_p2(&point5, &point4);
    ge_tobytes(&P_out, &point5);
    
    return true;
}
```

**鏁板鍏紡**:
```
P_out = Hs(8*r*A || n)*G + B
```

鍏朵腑 `Hs()` 鏄?hash-to-scalar 鍑芥暟.

### 2.4 鎵弿杈撳嚭 (鎺ユ敹鏂?

鎺ユ敹鏂规壂鎻忓尯鍧楅摼, 瀵规瘡绗斾氦鏄?

#### 姝ラ 1: 鎻愬彇浜ゆ槗鍏挜 R

```cpp
// src/cryptonote_basic/cryptonote_format_utils.cpp
crypto::public_key tx_pub_key = get_tx_pub_key_from_extra(tx);
```

#### 姝ラ 2: 閲嶅缓鍏变韩瀵嗛挜

```cpp
crypto::key_derivation derivation;
acc.get_device().generate_key_derivation(R, acc.m_view_secret_key, derivation);
// derivation = 8*a*R = 8*r*A (涓庡彂閫佹柟鐩稿悓)
```

#### 姝ラ 3: 閫愪釜妫€鏌ヨ緭鍑?

```cpp
for (size_t n = 0; n < tx.vout.size(); ++n) {
    crypto::public_key P_blockchain = get_output_public_key(tx.vout[n]);
    
    // 閲嶅缓鏈熸湜鐨勫叕閽?
    crypto::public_key P_expected;
    acc.get_device().derive_public_key(derivation, n, 
                                       acc.m_spend_public_key, 
                                       P_expected);
    
    if (P_blockchain == P_expected) {
        // 杩欎釜杈撳嚭灞炰簬鎴?
        outs.push_back(n);
    }
}
```

### 2.5 鑺辫垂杈撳嚭 (娲剧敓涓€娆℃€х閽?

```cpp
// src/crypto/crypto.cpp:L278 - derive_secret_key()
void derive_secret_key(const key_derivation &derivation,
                       size_t output_index,
                       const secret_key &b,      // 鑺辫垂绉侀挜
                       secret_key &p_out) {      // 涓€娆℃€х閽?
    ec_scalar scalar;
    
    derivation_to_scalar(derivation, output_index, scalar);
    sc_add(&p_out, &b, &scalar);  // p_out = Hs(...) + b
}
```

**鏁板楠岃瘉**:
```
P_out = p_out * G
      = (Hs(8*r*A || n) + b) * G
      = Hs(8*r*A || n)*G + b*G
      = Hs(8*r*A || n)*G + B  鉁?
```

### 2.6 View Tags 浼樺寲 (Monero v15+)

涓哄噺灏戞壂鎻忚绠楅噺 (256鍊嶆彁閫?:

```cpp
// src/crypto/crypto.cpp:L650 - derive_view_tag()
void derive_view_tag(const key_derivation &derivation,
                     size_t output_index,
                     view_tag &view_tag) {  // 鍙湁 1 瀛楄妭!
    struct {
        char salt[8];  // "view_tag"
        key_derivation derivation;
        char output_index_varint[...];
    } buf;
    
    memcpy(buf.salt, "view_tag", 8);
    buf.derivation = derivation;
    tools::write_varint(end, output_index);
    
    hash view_tag_full;
    cn_fast_hash(&buf, ..., view_tag_full);
    
    // 鍙彇鍓?1 瀛楄妭
    memcpy(&view_tag, &view_tag_full, 1);
}
```

**浼樺寲鍘熺悊**:
1. 鍙戦€佹柟璁＄畻 view tag 骞堕檮鍔犲埌杈撳嚭 (1瀛楄妭)
2. 鎺ユ敹鏂瑰厛妫€鏌?view tag (绠€鍗曟瘮杈?
3. 鍙湁鍖归厤 (1/256 姒傜巼) 鎵嶅仛瀹屾暣鐨?`derive_public_key`
4. 骞冲潎鍑忓皯 99.6% 鐨勮绠楅噺

### 2.7 鍏抽敭闂瑙ｇ瓟

鉁?**涓轰粈涔堥渶瑕佷袱瀵瑰瘑閽?**
- `a` (view key): 杞婚挶鍖?瀹¤鍛樺彲鎵弿, 浣嗕笉鑳借姳璐?
- `b` (spend key): 鍐烽挶鍖呬繚绠? 鍙湪绛惧悕鏃堕渶瑕?

鉁?**涓轰粈涔堣涔樹互 cofactor 8?**
- Ed25519 鏇茬嚎 cofactor=8, 纭繚缁撴灉鍦ㄧ礌鏁伴樁瀛愮兢
- 闃叉灏忓瓙缇ゆ敾鍑?(Lim-Lee attack)

鉁?**濡備綍闃叉鍦板潃閲嶇敤?**
- 姣忕瑪浜ゆ槗鐢熸垚鏂扮殑闅忔満 `r`
- 鍗充娇鍚屼竴鎺ユ敹鏂? 姣忎釜 `P_out` 閮戒笉鍚?
- 鍖哄潡閾惧垎鏋愭棤娉曞叧鑱旇緭鍑?

---

## 馃攽 3. Key Image (瀵嗛挜闀滃儚)

### 3.1 浣滅敤

**闃叉鍙岃姳鏀诲嚮**: Key Image 鏄粠绉侀挜娲剧敓鐨勫敮涓€鍊?鍏ㄧ綉鍙

- 姣忎釜杈撳嚭鏈夊敮涓€鐨?Key Image
- 鑺辫垂杈撳嚭鏃跺繀椤绘彁渚?Key Image
- 缃戠粶鎷掔粷閲嶅鐨?Key Image

### 3.2 鐢熸垚绠楁硶

```
杈撳叆: 
- x: 涓€娆℃€х閽?
- P: 瀵瑰簲鐨勪竴娆℃€у叕閽?(P = xG)

杈撳嚭:
- I: Key Image

璁＄畻:
I = x * Hp(P)

鍏朵腑 Hp(P) 鏄?"hash-to-point" 鍑芥暟
```

### 3.3 鍏抽敭鍑芥暟瀹炵幇

```cpp
// src/crypto/crypto.cpp:L620 - generate_key_image()
void generate_key_image(const public_key &pub,
                        const secret_key &sec,
                        key_image &image) {
    ge_p3 point;
    ge_p2 point2;
    
    // 璁＄畻 Hp(P)
    hash_to_ec(pub, point);  // 灏嗗叕閽ュ搱甯屽埌妞渾鏇茬嚎鐐?
    
    // 璁＄畻 x * Hp(P)
    ge_scalarmult(&point2, &sec, &point);
    ge_tobytes(&image, &point2);
}

// hash_to_ec 瀹炵幇:
static void hash_to_ec(const public_key &key, ge_p3 &res) {
    hash h;
    ge_p2 point;
    ge_p1p1 point2;
    
    // 鍝堝笇鍏挜
    cn_fast_hash(&key, sizeof(public_key), h);
    
    // 瑙ｇ爜鍝堝笇鍒扮偣 (Elligator)
    ge_fromfe_frombytes_vartime(&point, (const unsigned char*)&h);
    
    // 涔樹互 cofactor 8
    ge_mul8(&point2, &point);
    ge_p1p1_to_p3(&res, &point2);
}
```

**鏁板鎬ц川**:
- `I = x * Hp(P)` 缁戝畾鍒扮閽?`x`, 浣嗕笉娉勯湶 `x`
- 鍚屼竴杈撳嚭鍐嶆鑺辫垂浼氫骇鐢熺浉鍚岀殑 `I` (鍏ㄧ綉鍙娴?

### 3.4 瀹夊叏鎬?

**涓轰粈涔堜笉鑳界洿鎺ョ敤鍏挜?**
- 鍏挜浼氭硠闇茬幆绛惧悕涓殑鐪熷疄杈撳嚭
- Key Image 閫氳繃 hash-to-point 鎵撶牬鍏宠仈鎬?

**涓轰粈涔堟敾鍑昏€呬笉鑳戒吉閫?**
- 鍙湁鐭ラ亾绉侀挜 x 鎵嶈兘璁＄畻 I = x * Hp(P)
- 鐜鍚嶅悓鏃惰瘉鏄庣鍚嶈€呯煡閬撴煇涓閽?

---

## 锟?4. Bulletproofs 鑼冨洿璇佹槑

### 4.1 鍩烘湰姒傚康

**鐩殑**: 鍦ㄤ笉娉勯湶閲戦鐨勬儏鍐典笅, 璇佹槑浜ゆ槗閲戦 `v` 鍦ㄥ悎娉曡寖鍥村唴 (0 鈮?v < 2^64)銆?

**鏍稿績鎬濇兂**:
- Pedersen Commitment 闅愯棌閲戦: `C = v*H + gamma*G`
- Bulletproofs 璇佹槑 `v 鈭?[0, 2^N)` 涓斾笉娉勯湶 `v` 鎴?`gamma`
- 鑱氬悎璇佹槑: 澶氫釜杈撳嚭鍏变韩涓€涓瘉鏄? 鎸囨暟绾у噺灏戝ぇ灏?

**鍏抽敭鐗规€?*:
- **璇佹槑澶у皬**: 2*log鈧?n*m) + 9 涓き鍦嗘洸绾跨偣 (~700 bytes for 2 outputs)
- **楠岃瘉澶嶆潅搴?*: 鎵归噺楠岃瘉 O(n + m*log(m)), 鍗曚釜 O(n*log(n))
- **鏃犻渶鍙俊璁剧疆** (鐩告瘮 zk-SNARKs)

### 4.2 Pedersen Commitment 鍩虹

#### 鎵胯璁＄畻

```rust
// Pedersen Commitment: C = v*H + gamma*G
// v: 閲戦 (淇濆瘑)
// gamma: 鐩插寲鍥犲瓙 (闅忔満, 淇濆瘑)
// H, G: 鍩虹偣 (鍏紑)

let commitment = v * H + gamma * G;
```

**鍚屾€佹€?* (Homomorphic Property):
```
C鈧?+ C鈧?= (v鈧?H + 纬鈧?G) + (v鈧?H + 纬鈧?G)
        = (v鈧?+ v鈧?*H + (纬鈧?+ 纬鈧?*G
```

**搴旂敤**: 浜ゆ槗楠岃瘉
```
杈撳叆鎵胯涔嬪拰 = 杈撳嚭鎵胯涔嬪拰 + 鎵嬬画璐?H
危 C_in = 危 C_out + fee*H
```

### 4.3 Bulletproofs 璇佹槑鐢熸垚

Monero 浣跨敤鐨勬槸 **aggregated range proof** (澶氳緭鍑鸿仛鍚堣瘉鏄?銆?

#### 姝ラ 1: 鍒濆鍖?(PAPER LINES 41-44)

```cpp
// src/ringct/bulletproofs.cc:L800 - bulletproof_PROVE()

constexpr size_t N = 64;    // 姣旂壒浣嶆暟 (2^64 鑼冨洿)
size_t M = outputs.size();  // 杈撳嚭鏁伴噺
size_t MN = M * N;

// 灏嗛噾棰濈紪鐮佷负姣旂壒鍚戦噺 aL, aR
for (size_t j = 0; j < M; ++j) {
    for (size_t i = 0; i < N; ++i) {
        if (v[j] & (1 << i)) {
            aL[j*N + i] = 1;   // 姣旂壒涓?1
            aR[j*N + i] = 0;
        } else {
            aL[j*N + i] = 0;   // 姣旂壒涓?0
            aR[j*N + i] = -1;  // aR = aL - 1
        }
    }
}

// 鐢熸垚鎵胯 V = v*H + gamma*G
for (size_t i = 0; i < M; ++i) {
    V[i] = addKeys2(gamma[i] / 8, v[i] / 8, H);  // 闄や互8鏄痗ofactor澶勭悊
}
```

**aL, aR 鍏崇郴**:
```
aL[i] 鈭?{0, 1}         (姣旂壒鍊?
aR[i] = aL[i] - 1 鈭?{-1, 0}
aL 鈯?aR = 0            (Hadamard 绉负0)
危(aL[i] * 2^i) = v     (浜岃繘鍒堕噸寤洪噾棰?
```

#### 姝ラ 2: 鍚戦噺鎵胯 A, S (PAPER LINES 43-47)

```cpp
// 鐢熸垚闅忔満鍚戦噺 sL, sR (鐢ㄤ簬闆剁煡璇?
sL = random_vector(MN);
sR = random_vector(MN);

// A = aL*G + aR*H + alpha*G (绗竴涓壙璇?
alpha = random_scalar();
A = vector_exponent(aL, aR) + alpha * G;

// S = sL*G + sR*H + rho*G (绗簩涓壙璇? 鐢ㄤ簬澶氶」寮?
rho = random_scalar();
S = vector_exponent(sL, sR) + rho * G;
```

#### 姝ラ 3: Fiat-Shamir 鎸戞垬 (PAPER LINES 48-50)

```cpp
// 浠?V, A, S 娲剧敓鎸戞垬 (闈炰氦浜掑紡)
y = H(V || A || S)
z = H(y)
```

#### 姝ラ 4: 澶氶」寮忔瀯閫?(PAPER LINES 58-63)

```cpp
// 鏋勯€犲椤瑰紡 l(X), r(X)
// l(X) = (aL - z*1) + sL*X
// r(X) = y^n 鈯?(aR + z*1 + sR*X) + z虏*2^n

l0 = aL - z;
l1 = sL;

y_powers = [1, y, y虏, ..., y^(MN-1)];
r0 = (aR + z) 鈯?y_powers + z虏 * [2鈦? 2鹿, ..., 2^(N-1)];
r1 = y_powers 鈯?sR;

// 璁＄畻澶氶」寮忕郴鏁?
// t(X) = <l(X), r(X)> = t鈧€ + t鈧?X + t鈧?X虏
t1 = <l0, r1> + <l1, r0>;
t2 = <l1, r1>;
```

#### 姝ラ 5: 澶氶」寮忔壙璇?T1, T2 (PAPER LINES 52-53)

```cpp
tau1 = random_scalar();
tau2 = random_scalar();

T1 = t1*H / 8 + tau1*G / 8;
T2 = t2*H / 8 + tau2*G / 8;
```

#### 姝ラ 6: 鎸戞垬涓庡搷搴?(PAPER LINES 54-63)

```cpp
// 鏂版寫鎴?
x = H(z || T1 || T2);

// 璁＄畻 taux (鐩插寲鍥犲瓙鐨勭嚎鎬х粍鍚?
taux = tau1*x + tau2*x虏 + 危(z^(j+2) * gamma[j]);

// 璁＄畻 mu (鐢ㄤ簬鍐呯Н璇佹槑)
mu = x*rho + alpha;

// 璁＄畻 l, r (鍦?x 澶勬眰鍊?
l = l0 + l1*x;
r = r0 + r1*x;

// 璁＄畻 t (鍐呯Н)
t = <l, r>;
```

#### 姝ラ 7: 鍐呯Н璇佹槑 (Inner Product Argument)

杩欐槸 Bulletproofs 鐨勬牳蹇冮€掑綊绠楁硶:

```cpp
// src/ringct/bulletproofs.cc:L1100 - 鍐呯Н璇佹槑寰幆

nprime = MN;  // 鍒濆鍚戦噺闀垮害
L[], R[] = [];  // 宸﹀彸鎵胯鏁扮粍

while (nprime > 1) {
    nprime /= 2;
    
    // 璁＄畻浜ゅ弶鍐呯Н
    cL = <a[0:nprime], b[nprime:2*nprime]>;
    cR = <a[nprime:2*nprime], b[0:nprime]>;
    
    // 璁＄畻宸﹀彸鎵胯
    L = 危(a[0:nprime] * G[nprime:2*nprime]) 
      + 危(b[nprime:2*nprime] * H[0:nprime]) 
      + cL * x_ip * H;
      
    R = 危(a[nprime:2*nprime] * G[0:nprime]) 
      + 危(b[0:nprime] * H[nprime:2*nprime]) 
      + cR * x_ip * H;
    
    // Fiat-Shamir 鎸戞垬
    w = H(L || R);
    
    // 鎶樺彔鍚戦噺 (閫掑綊鍘嬬缉)
    a' = w*a[0:nprime] + w鈦宦?a[nprime:2*nprime];
    b' = w鈦宦?b[0:nprime] + w*b[nprime:2*nprime];
    
    // 鎶樺彔鍩虹偣
    G' = w鈦宦?G[0:nprime] + w*G[nprime:2*nprime];
    H' = w*H[0:nprime] + w鈦宦?H[nprime:2*nprime];
    
    a = a'; b = b'; G = G'; H = H';
}

// 鏈€缁堣繑鍥炴爣閲?a, b (闀垮害涓?)
```

**璇佹槑缁撴瀯**:
```rust
struct Bulletproof {
    V: Vec<Point>,       // 鎵胯鍚戦噺 (M涓?
    A: Point,            // 鍚戦噺鎵胯 A
    S: Point,            // 鍚戦噺鎵胯 S
    T1: Point,           // 澶氶」寮忔壙璇?T1
    T2: Point,           // 澶氶」寮忔壙璇?T2
    taux: Scalar,        // 鐩插寲鍥犲瓙
    mu: Scalar,          // 鍐呯Н鐩插寲鍥犲瓙
    L: Vec<Point>,       // 宸︽壙璇?(log鈧?MN)涓?
    R: Vec<Point>,       // 鍙虫壙璇?(log鈧?MN)涓?
    a: Scalar,           // 鏈€缁?a
    b: Scalar,           // 鏈€缁?b
    t: Scalar,           // 鏈€缁堝唴绉?
}
```

**澶у皬璁＄畻**:
```
M=2 outputs (128 bits total):
- V: 2 * 32 = 64 bytes
- A, S, T1, T2: 4 * 32 = 128 bytes
- taux, mu, a, b, t: 5 * 32 = 160 bytes
- L, R: 2 * log鈧?128) * 32 = 2 * 7 * 32 = 448 bytes
Total: ~800 bytes

瀵规瘮: 鍘熷 RingCT (non-Bulletproofs): ~7 KB
鑺傜渷: 89% 绌洪棿
```

### 4.4 Bulletproofs 楠岃瘉

楠岃瘉鏂圭▼ (PAPER LINE 62-65):

```cpp
// src/ringct/bulletproofs.cc:L1400 - bulletproof_VERIFY()

// 閲嶅缓鎸戞垬
y = H(V || A || S);
z = H(y);
x = H(z || T1 || T2);
x_ip = H(x || taux || mu || t);
w[] = [H(L[0] || R[0]), H(L[1] || R[1]), ...];

// 楠岃瘉鏂圭▼ 1: Pedersen Commitment 骞宠　
// g^(-z) * h^(z*y^n + z虏*2^n) * V鈧乛(z虏) * V鈧俕(z鈦? == g^(-taux) * h^t * T1^x * T2^(x虏)

lhs = -z*G + (z*y^n + z虏*2^n)*H 
    + z虏*V[0] + z鈦?V[1]  // 澶氳緭鍑鸿仛鍚?
    
rhs = -taux*G + t*H + x*T1 + x虏*T2;

CHECK(lhs == rhs);  // 鏂圭▼ 1

// 楠岃瘉鏂圭▼ 2: 鍐呯Н璇佹槑
// 閲嶅缓鍩虹偣 G', H'
for (i = 0; i < log鈧?MN); i++) {
    G' = w[i]鈦宦?G[left] + w[i]*G[right];
    H' = w[i]*H[left] + w[i]鈦宦?H[right];
}

// 妫€鏌ユ渶缁堝唴绉?
lhs = a*G' + b*H' + (a*b)*x_ip*H;
rhs = mu*G + 危(w[i]虏*L[i]) + 危(w[i]鈦宦?R[i]) + ...;

CHECK(lhs == rhs);  // 鏂圭▼ 2
```

### 4.5 鎵归噺楠岃瘉浼樺寲

Monero 鏀寔鎵归噺楠岃瘉澶氫釜 Bulletproofs (鍖哄潡涓墍鏈変氦鏄?:

```cpp
// src/ringct/bulletproofs.cc:L1300 - bulletproof_VERIFY(batch)

// 涓烘瘡涓瘉鏄庣敓鎴愰殢鏈烘潈閲?
weight_y[] = random();
weight_z[] = random();

// 鑱氬悎鎵€鏈夐獙璇佹柟绋?(鍔犳潈鍜?
aggregate_lhs = 危(weight_y[i] * lhs[i]) + 危(weight_z[i] * lhs2[i]);
aggregate_rhs = 危(weight_y[i] * rhs[i]) + 危(weight_z[i] * rhs2[i]);

// 鍗曟澶氭爣閲忎箻娉曟鏌?
CHECK(aggregate_lhs == aggregate_rhs);
```

**鎬ц兘鎻愬崌**:
- 鍗曚釜楠岃瘉: ~5ms (1 output)
- 鎵归噺楠岃瘉 1000 proofs: ~1.2s (骞冲潎 1.2ms/proof)
- **鎻愰€?*: 4鍊?

### 4.6 鍏抽敭闂瑙ｇ瓟

鉁?**涓轰粈涔堥渶瑕佷袱涓悜閲?aL, aR?**
- `aL 鈭?{0,1}` 琛ㄧず姣旂壒鍊?
- `aR = aL - 1 鈭?{-1,0}` 纭繚 `aL 鈯?aR = 0`
- 杩欎釜绾︽潫闅愬紡璇佹槑浜?aL 鏄簩杩涘埗

鉁?**涓轰粈涔堣閫掑綊鎶樺彔?**
- 鍒濆鍚戦噺闀垮害 MN (渚嬪 128)
- 姣忚疆鎶樺彔鍑忓崐: 128 鈫?64 鈫?32 鈫?... 鈫?1
- 璇佹槑澶у皬: O(log MN) 鑰岄潪 O(MN)

鉁?**Bulletproofs vs Bulletproofs+?**
- Bulletproofs+: Monero v15+ 浣跨敤
- 鏀硅繘: 鍑忓皯 1 涓爣閲?(weighted norm argument)
- 鑺傜渷: ~32 bytes/proof

鉁?**濡備綍闃叉璐熸暟?**
- 鑼冨洿璇佹槑寮哄埗 `v 鈭?[0, 2^N)`
- 璐熸暟鐨勪簩杩涘埗琛ㄧず浼氭孩鍑?N 浣?
- 楠岃瘉鏂圭▼浼氬け璐?

---

## 锟金煍?5. RingCT 瀹屾暣浜ゆ槗娴佺▼

### 4.1 浜ゆ槗缁撴瀯

```cpp
// src/ringct/rctTypes.h
struct rctSig {
    rctSigBase base;        // 鍩虹绛惧悕
    vector<clsag> p;        // 鐜鍚?(姣忎釜杈撳叆涓€涓?
    vector<rangeSig> rangeSigs; // 鑼冨洿璇佹槑 (Bulletproofs)
    // ...
};
```

### 4.2 浜ゆ槗鏋勯€犳楠?

**TODO**: 璇︾粏鍒嗘瀽 `construct_tx_and_get_tx_key()`

```
1. 閫夋嫨杈撳叆 (UTXOs)
2. 涓烘瘡涓緭鍏ラ€夋嫨 ring members
3. 鐢熸垚杈撳嚭鐨勯殣韬湴鍧€
4. 鐢熸垚 Pedersen Commitments (闅愯棌閲戦)
5. 鐢熸垚 Bulletproofs (璇佹槑閲戦 鈮?0)
6. 鐢熸垚 CLSAG 绛惧悕 (璇佹槑鎷ユ湁鏌愪釜杈撳叆)
7. 楠岃瘉 Commitment 骞宠　 (杈撳叆 = 杈撳嚭 + 鎵嬬画璐?
```

### 4.3 鎵胯鏂规

**Pedersen Commitment**:
```
C(a, r) = aH + rG

鍏朵腑:
- a: 閲戦 (secret)
- r: 鐩插洜瀛?(blinding factor)
- H, G: 鍩虹偣
```

**骞宠　楠岃瘉**:
```
sum(C_inputs) = sum(C_outputs) + fee * H
```

### 4.4 鍏抽敭闂

- [ ] Ring size 濡備綍閫夋嫨? (褰撳墠榛樿 16)
- [ ] Ring members 閫夋嫨绠楁硶? (gamma 鍒嗗竷)
- [ ] Bulletproofs 鑱氬悎濡備綍宸ヤ綔?
- [ ] 鎵嬬画璐瑰浣曡绠?

---

## 馃搳 5. 鎬ц兘鏁版嵁

### 5.1 绛惧悕/楠岃瘉鏃堕棿

**TODO**: 杩愯 Monero 鍩哄噯娴嬭瘯

```bash
cd d:\WEB3_AI寮€鍙慭monero-research
# 缂栬瘧骞惰繍琛屾€ц兘娴嬭瘯
```

**棰勬湡鏁版嵁** (Ring Size = 16):
- 绛惧悕鐢熸垚: ~50-100ms
- 绛惧悕楠岃瘉: ~5-10ms
- Bulletproofs 鐢熸垚: ~200-300ms
- Bulletproofs 楠岃瘉: ~5-10ms (鎵归噺楠岃瘉鏇村揩)

### 5.2 浜ゆ槗澶у皬

| Ring Size | CLSAG Size | Bulletproofs | 鎬诲ぇ灏?(浼扮畻) |
|-----------|-----------|--------------|---------------|
| 11 | ~1.5 KB | ~1.5 KB | ~3 KB |
| 16 | ~2 KB | ~1.5 KB | ~3.5 KB |
| 64 | ~7 KB | ~1.5 KB | ~8.5 KB |

---

## 馃幆 6. 搴旂敤鍒?SuperVM

### 6.1 璁捐鍐崇瓥

**闇€瑕佸喅瀹?*:
1. **Ring Size**: Monero 鐢?16, 鎴戜滑鐢ㄥ灏?
   - 鏇村ぇ = 鏇村尶鍚? 浣嗘洿鎱?鏇村ぇ
   - 寤鸿: 11 (榛樿), 鏀寔 3-64 鍙厤缃?

2. **绛惧悕绠楁硶**: CLSAG vs MLSAG?
   - 閫夋嫨: CLSAG (鏇村揩, 鏇村皬)

3. **Range Proof**: Bulletproofs vs Bulletproofs+?
   - 閫夋嫨: Bulletproofs (curve25519-dalek 宸叉敮鎸?

4. **zkSNARK**: 鏄惁棰濆闆嗘垚?
   - 寰呰瘎浼? Week 3-4 鍐崇瓥

### 6.2 API 璁捐鑽夋

```rust
// 鍩轰簬 Monero 瀛︿範鎴愭灉璁捐

pub struct RingSigner {
    ring_size: usize,
    ring_members: Vec<PublicKey>,
    secret_index: usize,
    secret_key: SecretKey,
}

impl RingSigner {
    pub fn sign(&self, message: &[u8]) -> Result<RingSignature> {
        // 1. 鐢熸垚 Key Image
        let key_image = self.generate_key_image();
        
        // 2. 鎵ц CLSAG 绛惧悕绠楁硶
        // (鍙傝€?Monero proveRctCLSAGSimple)
        
        todo!()
    }
}
```

### 6.3 瀹炵幇璺嚎鍥?

**Week 9-12** (Phase 2.2.1):
- [ ] 瀹炵幇 `generate_key_image()` (鍩轰簬 Monero)
- [ ] 瀹炵幇 CLSAG 绛惧悕绠楁硶
- [ ] 瀹炵幇 CLSAG 楠岃瘉绠楁硶
- [ ] 鎬ц兘娴嬭瘯: 鐩爣 <50ms 绛惧悕, <5ms 楠岃瘉

---

## 馃摎 7. 鍙傝€冭祫鏂?

### 7.1 蹇呰璁烘枃

- [ ] **Zero to Monero 2.0** (450 椤靛畬鏁存暀绋?
  - https://web.getmonero.org/library/Zero-to-Monero-2-0-0.pdf
  - 绔犺妭閲嶇偣: Ch3 (Ring Signatures), Ch4 (Stealth Addresses), Ch5 (RingCT)

- [ ] **CryptoNote v1.0 Whitepaper**
  - https://cryptonote.org/whitepaper.pdf
  - 鍘熷闅愮甯佽璁?

- [ ] **Triptych: Logarithmic-Sized Linkable Ring Signatures**
  - https://eprint.iacr.org/2020/018
  - 涓嬩竴浠ｇ幆绛惧悕绠楁硶

### 7.2 Monero 鏂囨。

- **瀹樻柟鏂囨。**: https://www.getmonero.org/resources/developer-guides/
- **Moneropedia**: https://www.getmonero.org/resources/moneropedia/
- **StackExchange**: https://monero.stackexchange.com/

### 7.3 浠ｇ爜瀵艰埅

**鏍稿績鐩綍**:
```
monero-research/
鈹溾攢鈹€ src/
鈹?  鈹溾攢鈹€ ringct/              鈫?RingCT 瀹炵幇 (閲嶇偣!)
鈹?  鈹?  鈹溾攢鈹€ rctSigs.cpp      鈫?CLSAG/MLSAG 绛惧悕
鈹?  鈹?  鈹溾攢鈹€ rctTypes.h       鈫?鏁版嵁缁撴瀯
鈹?  鈹?  鈹斺攢鈹€ bulletproofs.cc  鈫?鑼冨洿璇佹槑
鈹?  鈹溾攢鈹€ crypto/              鈫?鍩虹瀵嗙爜瀛?
鈹?  鈹?  鈹溾攢鈹€ crypto.cpp       鈫?Ed25519, Key Image
鈹?  鈹?  鈹斺攢鈹€ hash-ops.h       鈫?鍝堝笇鍑芥暟
鈹?  鈹溾攢鈹€ cryptonote_basic/    鈫?CryptoNote 鏍稿績
鈹?  鈹?  鈹斺攢鈹€ cryptonote_format_utils.cpp 鈫?鍦板潃鐢熸垚
鈹?  鈹斺攢鈹€ wallet/              鈫?閽卞寘閫昏緫
鈹?      鈹斺攢鈹€ wallet2.cpp      鈫?浜ゆ槗鏋勯€? 鎵弿
```

---

## 鉁?瀛︿範杩涘害

### Week 1 (2025-11-04 鑷?2025-11-10)

**Day 1 (2025-11-04)**:
- [x] 鍏嬮殕 Monero 浠撳簱
- [x] 鍒涘缓瀛︿範绗旇妗嗘灦
- [ ] 闃呰 `rctTypes.h` 浜嗚В鏁版嵁缁撴瀯
- [ ] 闃呰 Zero to Monero Ch3 (Ring Signatures)

**Day 2-3**:
- [ ] 娣卞叆 `rctSigs.cpp` - CLSAG 瀹炵幇
- [ ] 鎻愬彇鍏抽敭浠ｇ爜鐗囨鍒扮瑪璁?
- [ ] 鐢诲嚭 CLSAG 绛惧悕娴佺▼鍥?

**Day 4-5**:
- [ ] 鐮旂┒ Stealth Address 瀹炵幇
- [ ] 鐮旂┒ Key Image 鐢熸垚
- [ ] 杩愯 Monero 娴嬭瘯鐢ㄤ緥

**Day 6-7**:
- [ ] 鎬荤粨 Week 1 瀛︿範鎴愭灉
- [ ] 鍑嗗 Week 2 娣卞叆鐮旂┒璁″垝

### Week 2 (2025-11-11 鑷?2025-11-17)

**Day 8-10**:
- [ ] 鐮旂┒ Bulletproofs 瀹炵幇
- [ ] 鐮旂┒ RingCT 瀹屾暣浜ゆ槗娴佺▼
- [ ] 缂栧啓 C++ 娴嬭瘯浠ｇ爜楠岃瘉鐞嗚В

**Day 11-12**:
- [ ] 璁捐 SuperVM 鐨?Ring Signature API
- [ ] 缂栧啓鎶€鏈€夊瀷鎶ュ憡
- [ ] 纭畾瀹炵幇缁嗚妭 (ring size, 绠楁硶閫夋嫨)

**Day 13-14**:
- [ ] 瀹屾垚 Monero 瀛︿範鎬荤粨鎶ュ憡
- [ ] 鍑嗗 Week 3 zkSNARK 璇勪及

---

## 馃挕 闂涓庢€濊€?

### 寰呰В鍐抽棶棰?

1. **Ring Member 閫夋嫨绠楁硶**
   - Monero 浣跨敤 gamma 鍒嗗竷閫夋嫨 decoys
   - 濡備綍闃叉缁熻鍒嗘瀽鏀诲嚮?

2. **鎬ц兘浼樺寲**
   - 鎵归噺楠岃瘉濡備綍瀹炵幇?
   - 鏄惁闇€瑕侀璁＄畻琛?

3. **瀛樺偍浼樺寲**
   - Key Image 绱㈠紩缁撴瀯?
   - 濡備綍楂樻晥妫€娴嬪弻鑺?

### 涓汉鐞嗚В

**TODO**: 姣忓ぉ璁板綍瀛︿範蹇冨緱

---

## 馃敆 鐩稿叧绗旇

- `curve25519-dalek-notes.md` (Week 1-2 骞惰瀛︿範)
- `cryptonote-whitepaper-notes.md` (Week 1-2 骞惰瀛︿範)
- `phase2-implementation-decisions.md` (Week 7-8 鏋舵瀯璁捐)

---

**鏈€鍚庢洿鏂?*: 2025-11-04  
**涓嬫鏇存柊**: 姣忔棩鍚屾瀛︿範杩涘害




