// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MultiplyVerifier {
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
    G1Point constant ALPHA = G1Point(0x1188d809b67f229cb41db2be9f6e4c6ff68c58bc69602c7524d524d28115a5b829904409ac08c9fe3d568a3a0db5e5c3, 0x0196c67bd6d99c839a9c4676beede9cfbf0cce89c2cc97837815741ef9ad435d1e1aefc9f4752a58cd1d83df183aa144);
    G2Point constant BETA = G2Point([0x099faa013a58e88cb85124af300ee59b4a5b437ad295839b77a19ae5d7e72a6bf73904f0236635f1c9a5d03f5386462a, 0x1173b661e4b80e5f8929ecb7511e25751037d394231d36c0bbee48fcefd1c52b48a7147b0d6b8145b3db8262f8aa4c0e], [0x041adca5ff7a4a3034fc9c65266aeed508241039ded868e3a9d0178d89a93799422c9f6d990e9959e82d9638b8207253, 0x059e1465aa54c3e1e699b93a76eddaf1731b0d7561d2339d7b212b5cac735430f1a9a24f244944f14b2a0eb1a0b7ec88]);
    G2Point constant GAMMA = G2Point([0x19a7a0bee6fd22ac88ad4c9ad1cb208403d14f6f5370d7686cc50566e61ab5478611753cc2f1bb93021c50aa85a5514c, 0x198c46e98492af9d5ef7c4200e4bba2d4e9b1882e97273c14ec826b6c087e5919774ff12ef7b8ca15baa24db8ce3c3cc], [0x02be7754b2bd5c1d621c75a012a0477ad0df48f1416b7822f7e50bf1aaee6a26ac1c77a614712a3780d2fe94786484e3, 0x18ad94bcf2778a3b433ce0d188d4ba42f04a2f62b68313a92a5cec247893d8437e2016147be487897c065b6f2753dd87]);
    G2Point constant DELTA = G2Point([0x19ca603ef349d222091000d4956f8a2ab12c255d1e636879316cb5ae9bddc57123d231038efb5532f9623b950dbf1389, 0x0c62fb54b88f2ca259677581b02f75a786f00fd36377db2ba70824eb156dbe575c2ced23f3a4aadd235c7fca58560ce6], [0x15d678cc51379423dbe0e516e96f4a1ecbb5d9f68b74ba0b607619db235c467153c6722c956e9682723d6abc732ca907, 0x0af76e25521cd9725423e82e871e3027b534f8c1324366e74fb2ff6682de5f72b652543491511c6a29f8b9ab9da50924]);

    // Public inputs: 1 (gamma_abc inline expansion in verifyProof)

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
        uint256[1] calldata input
    ) external view returns (bool) {
        G1Point memory proofA = G1Point(a[0], a[1]);
        G2Point memory proofB = G2Point([b[0][0], b[0][1]], [b[1][0], b[1][1]]);
        G1Point memory proofC = G1Point(c[0], c[1]);

        // Compute vk_x = gamma_abc[0] + sum(input[i] * gamma_abc[i+1])
        G1Point memory vkX = G1Point(0,0);
        // vk_x = gamma_abc[0] + Σ input[i] * gamma_abc[i+1]
        // 以下点常量由生成器在编译期写入（Rust 侧内联）
        // __GAMMA_ABC_INLINE_START__
        vkX = pointAdd(vkX, G1Point(0x1717dac08883bcb233861595664b8d3d6f730d3f98b72c2e6a031a333277dfb4ea52457d8fe956d429a8c8d2b77b33b2, 0x0c8aa56783cd0e350298c277e44c8ce710a22d82ed9c445b4cd84daa623b87549dc074e5330050230994c3bc747d4c16));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x05b5d37188da35f8c3e9588887ca01686d456840f693d2680a5120a0dbf7a4e2896a17fb627823db1546e133afad259d, 0x0d15f573d28485cc7dabc9c231a600101600a35c667d48ed5b107cae265a4a04d5d98292c7571a12fbb787131e7e40e4), input[0]));
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
