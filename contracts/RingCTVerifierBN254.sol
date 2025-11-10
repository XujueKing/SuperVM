// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract RingCTVerifierBN254 {
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
    G1Point constant ALPHA = G1Point(0x16cde7672d240777d5e1e109af2a17cf9c6f65aabd05a76d82461ed92edabcfb, 0x21d9f38f3d392cb6605fe0986b27e2ce9c23dc0b8936d7af05eb213dfdaed020);
    G2Point constant BETA = G2Point([0x2fa32830b84e4463e4dede31a16278eb742416eae4bef6b156903587cff98380, 0x135410eb08cc99678e973f836aac67573b37106e8f6b65ddc7b2976fc16ad2eb], [0x1788fe169ad482a20fd2c4a70f3c65a55fed2022df58aca286c1acdf8c34c5f9, 0x1489d87c0678fe268212b8a9f9e57aa5ee218a972b6aa066a6d9229f4fea9b99]);
    G2Point constant GAMMA = G2Point([0x2cf2bb37e9ec0305b71e42f5cfb5a6117a30ba1086fa0600a9720def28ba4ee7, 0x2bbec1646afc4b6f9eaa3926da6b62021e69922d4f2ffcacf2896a7bb636d4a9], [0x062f3d3b4d162be8243a46c651e1fe735841bcaed0b7b2ec9d4979fa97d48ff0, 0x0d1b2b86dac682bd0e36ee8de9c63750ffeeb520c92681976e362d1b3dc0e85d]);
    G2Point constant DELTA = G2Point([0x01135fa9c047229ca25627c2bf2194c043e03f34a8f6cc94b5ff1d61f19b9a7e, 0x23391ed97851cfb5d0bb6bc901ec00feaa36235fac6a8dba43d0ee75e12c2aa2], [0x2d270a276a2590d9e1b2b8bd70447b7880e92942981c2b0136ac58bafb389d87, 0x137ac003c9aff19bd67336083096af7eee88535985cbb6e0b40d74e8404559d4]);

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

    // Negate a G1 point
    function negate(G1Point memory p) internal pure returns (G1Point memory) {
        uint256 q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;
        if (p.X == 0 && p.Y == 0) {
            return G1Point(0, 0);
        }
        return G1Point(p.X, q - (p.Y % q));
    }

    // Add two G1 points using precompile 0x06
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

    // Scalar multiplication using precompile 0x07
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
        // __GAMMA_ABC_INLINE_START__
        vkX = pointAdd(vkX, G1Point(0x0405c675f9ce207bd9940db7d6e495287cc078bdfe2577eebdb9faf525b45d08, 0x22869ebd8051833f16b1caa971740ad6793ebd2467a03f557bcefd3d20daf206));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x0159d434e826fbc157548933238bd548e00921094daec69a6f2aeeb74d6bece0, 0x2907b982d9565176d4f939fbaaea4fc3984cdf39aa3ff06992a956623768d8f2), input[0]));
        // __GAMMA_ABC_INLINE_END__

        return pairing(
            negate(proofA), proofB,
            vkX, GAMMA,
            proofC, DELTA,
            ALPHA, BETA
        );
    }

}
