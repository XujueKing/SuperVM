// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract RingCTVerifier {
    // Pairing library structures
    struct G1Point {
        uint256 X;
        uint256 Y;
    }

    struct G2Point {
        uint256[2] X;
        uint256[2] Y;
    }

    // Verification Key
    G1Point constant ALPHA = G1Point(0x04510522ac03258663488afe1f1d298a6d9b4abc21d5114fc12b1c752202e871a7a2f697c46aaabf7c57ce9648de1c0f, 0x086ae4e6b80e5b3fbe54b44c5ded22142d201e381cf6994217ae65ec9bbff177591b33badfad559ee7798bb641e1d834);
    G2Point constant BETA = G2Point([0x0a42f0834e0a4901ee20c65306d4b74b52e29ccc4956c275fdd7e75c3052c5bdd9c44cadda3e7c1333e2dffbcab56a00, 0x0ec2ef88502181fbb5fbe33a9b0e0111b0c3997b9ab34cc456c00337729099699e9638e51e0b5c6be1847a8ae0d02e64], [0x03784c39d0af7abff800e045f941215e2f4612bfa11388de3958ec4df0c5b0455d950dd1601ff52e7021484eabc0b751, 0x1091f30061ae9ef07e1885719c2e30c455297553f1351d7435cc2a01cfdc6f5d5b11336cf10f079020d6479b27e7649c]);
    G2Point constant GAMMA = G2Point([0x0b55c65e02d694a70b1c44f6c5ad50792c68e248e34950471348927a0f2cc5e8ea671f6e5e7b6b746423678c7badd94b, 0x0d1b0404ea106640d8815dd136934ce24c286c7a05d18f5b5f1c7cf02b235a107b0b5b91f045d96690641e17ea47d7c5], [0x0fd67b1198fa3fab7cde52898c6682a7014adead8553f4f699513b836fbd3834b0931a44c06257ce665f69585a667e68, 0x080e3ee9909e82aa48fc09a678c4693c5dac90885a45d2f4e4187344d96dd019f29d1f01248846c33630fb6a3bb731ad]);
    G2Point constant DELTA = G2Point([0x09532e0e45dba074aa29eaf51900b4174b84a6559faa6209936d9b045edc5d3bef0ab7879341bf5c994bd0a51c38a3d7, 0x09ad246fc713e9b87f2c8b9cc7970567b23fa9f09243a2f8ee6da46a9790e81f052ba3cc468ed4507a1dfa3f307c298c], [0x000d459a4393b72b11ca88b2fe592e9395541ac5e7a73c42dc579db7d511b0854ac8504cc1d5147526b1953fe669dd9e, 0x04d3060f28a331a3992ffadd75656a7df223af51d3a632f3ae85b4c71a276c18b781eccca2971bb0866173a6bafd80b3]);

    // Public inputs: 8 (gamma_abc inline expansion in verifyProof)

    // Pairing check using precompile
    function pairing(
        G1Point memory a1,
        G2Point memory a2,
        G1Point memory b1,
        G2Point memory b2,
        G1Point memory c1,
        G2Point memory c2,
        G1Point memory d1,
        G2Point memory d2
    ) internal view returns (bool) {
        G1Point[4] memory p1 = [a1, b1, c1, d1];
        G2Point[4] memory p2 = [a2, b2, c2, d2];
        uint256 inputSize = 24;
        uint256[] memory input = new uint256[](inputSize);

        for (uint256 i = 0; i < 4; i++) {
            uint256 j = i * 6;
            input[j + 0] = p1[i].X;
            input[j + 1] = p1[i].Y;
            input[j + 2] = p2[i].X[0];
            input[j + 3] = p2[i].X[1];
            input[j + 4] = p2[i].Y[0];
            input[j + 5] = p2[i].Y[1];
        }

        uint256[1] memory out;
        bool success;

        assembly {
            success := staticcall(
                sub(gas(), 2000),
                0x08,  // Precompile address for pairing check
                add(input, 0x20),
                mul(inputSize, 0x20),
                out,
                0x20
            )
        }

        require(success, "Pairing check failed");
        return out[0] != 0;
    }

    // Main verification function
    function verifyProof(
        uint256[2] calldata a,
        uint256[2][2] calldata b,
        uint256[2] calldata c,
        uint256[8] calldata input
    ) external view returns (bool) {
        G1Point memory proofA = G1Point(a[0], a[1]);
        G2Point memory proofB = G2Point([b[0][0], b[0][1]], [b[1][0], b[1][1]]);
        G1Point memory proofC = G1Point(c[0], c[1]);

        // Compute vk_x = gamma_abc[0] + sum(input[i] * gamma_abc[i+1])
        G1Point memory vkX = G1Point(0,0);
        // vk_x = gamma_abc[0] + Σ input[i] * gamma_abc[i+1]
        // 以下点常量由生成器在编译期写入（Rust 侧内联）
        // __GAMMA_ABC_INLINE_START__
        vkX = pointAdd(vkX, G1Point(0x0f1078d39db577af445c4dd9c8b2979b3f60f029c214668b1d39128cc13268380e678391a7c86c9021475b6b1b80da77, 0x0ca8909b770eecb39cce6e167dcc3e4bb5a97987b38fab09dd121d2839d6d7b177d3d417e897f99ea006e2456efdc97b));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0c1540203fae4cb914898ac9088d8539dc9b9cc1d7bb7c36ee7bdb7b1082307d7603f4d2f7305c2607dec8194795a6d0, 0x19bb34973ffa068a84b0971a89338daf8f99c6d575c0281011224221094f06741504b2465bb7a4deaa8a3f2ba56f3107), input[0]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x165486fd25f121a6fd2d54beb9856e50c657182703dc46ce7cc1dbb740d00a178d83e1345cd39343b8607d42bce715c9, 0x171bc162406911a7ec90d90c5c648bf0e5b5663b3cb038d00549512f569c1625281d98e7009bb4edeefa9415f27ab721), input[1]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x068f41d72467f962413f89fd7b25989ada47abeb17a46ba9005c024b4ea2f8fc2b40c0927feaa1d7db76d59584c08291, 0x17bcc8fd026f1bc98360eeefd50a6cc621dd4f0dc810766070e2af58fc0462139be7b7ce02c3287328fc6a7d18b3a79e), input[2]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0cfce9e79e27d8308df419b4903cea55af7729cbcf5049e24b9b9599754a1071d4dc660a0525df990d9e637fe945ecae, 0x065e6a9e17e43ef1532eb7338410de64bc295bee1f5b9a4bb299060b122d310300a057d06173bb5fcb06f09dc84b755a), input[3]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0bf6190f42728fc17430f04c2f9ee64435b92dba37ec91f71da881ee3eab7e7cc6578c7b801e0d8ce21c47085a23ba04, 0x06e90cc1e642729b1bbcfa345af5a642ec783b6b8a3cd6cd73de4670b1308692b3006a7d4d8775754fcef677370dca71), input[4]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0c824154888b5828e9481ea6a1fd4ac42104696c3fcc1c12a32ca37fa34c14793e3024b980d24615ea5a30891fd81112, 0x07e0775df0517b907627124cec1e0cbc08af589f57cffee6739586da3d56f612bfa0ecf1545bb9560eb804e2c5aebca0), input[5]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0777121b718335a8030a92e11292ffe433f06540b2d81ae1a2cb4ad673624d72841e06221d28f935d8c1526407fc3d71, 0x178c7049e704c88452dffb1dc991ee38aa46dd82bbaf0d458dca4317dea3498ca26bd8e323b5125e33edff0521daa95e), input[6]));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0b9dea7f659b6314e9ae9ce1abf6a43a128637109beb7d05e20f5b1c3fa88df512ada498a220639eceb7ae455955873e, 0x0044d79bcaec5cc1f138632c44e54dab67d99b7be18e3ffa4d1b1838e0af6a3c82424d3fdc7fa0027bc7dbcdb55388aa), input[7]));
        // __GAMMA_ABC_INLINE_END__

        // Pairing check: e(A, B) = e(alpha, beta) * e(vk_x, gamma) * e(C, delta)
        // Rearranged: e(A, B) * e(-vk_x, gamma) * e(-C, delta) = e(alpha, beta)
        return pairing(
            negate(proofA), proofB,
            vkX, GAMMA,
            proofC, DELTA,
            ALPHA, BETA
        );
    }

    // Helper: G1 point addition using precompile
    function pointAdd(G1Point memory p1, G1Point memory p2) internal view returns (G1Point memory) {
        uint256[4] memory input;
        input[0] = p1.X;
        input[1] = p1.Y;
        input[2] = p2.X;
        input[3] = p2.Y;
        bool success;
        G1Point memory result;
        assembly {
            success := staticcall(sub(gas(), 2000), 0x06, input, 0x80, result, 0x40)
        }
        require(success, "Point addition failed");
        return result;
    }

    // Helper: G1 scalar multiplication using precompile
    function scalarMul(G1Point memory p, uint256 s) internal view returns (G1Point memory) {
        uint256[3] memory input;
        input[0] = p.X;
        input[1] = p.Y;
        input[2] = s;
        bool success;
        G1Point memory result;
        assembly {
            success := staticcall(sub(gas(), 2000), 0x07, input, 0x60, result, 0x40)
        }
        require(success, "Scalar multiplication failed");
        return result;
    }

    // Helper: Negate G1 point
    function negate(G1Point memory p) internal pure returns (G1Point memory) {
        uint256 q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;
        if (p.X == 0 && p.Y == 0) return G1Point(0, 0);
        return G1Point(p.X, q - (p.Y % q));
    }
}
