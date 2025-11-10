// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract BN254MultiplyVerifier {
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
    G1Point constant ALPHA = G1Point(0x0cbcfcc6ec8cbdc4dba224a4b8d9b6bd6fb89df34221166f580975abfb92afcc, 0x06b404c906a84af3cef06cf8eabd2fde8b324cc9ba4111f83d0865d117f4c6d7);
    G2Point constant BETA = G2Point([0x2660cb78fd1893bc62658cd1f3d6c16a52a91cca2d468396b2a6203a7c963336, 0x1229c422b86a886cb50250b035f34d15a50e71da2ecf8b50ef8ab527392f69e4], [0x0877cb4dd51404ac6a3314c9fc223353156467d6be8705dcb12f659a32352318, 0x0533b631b811e7210d4d93a2d9b05e1e5938467526dc0b5e846b1ca66c593140]);
    G2Point constant GAMMA = G2Point([0x16705a32b022683812eca4283fa6a3347fef7a8e72b20a4527ec4d4e286233a5, 0x05e61e66fd7c7210ce1e3300cba8e965d716858665473866792f7e7563b918b5], [0x199c8592e5ac2a71fb79322aa377918ef836648cab38828c998d399c07f0c09e, 0x26cc46ded0bd6977a18dee4eb818b781f3afddc372977e7dee5d530537850a85]);
    G2Point constant DELTA = G2Point([0x1da4a9663bce7af967a9f03d016729ef5f589e3d0e8394468f35cd5c80505883, 0x20a6809e1c55e4892296d70b49be008dd5882dcd37b656bdf4a67f53ebedb224], [0x1235c2140d719b1a4c2c7e841e35d7e5b768703e5d660682d0e184d5d5909ba6, 0x09810d46255f01195d966f332962d6f438390a30577578fe8a2d21d295d8fb4d]);

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
        vkX = pointAdd(vkX, G1Point(0x0d77b74156186cacbf82fb2c7929cecd1bd1dbf6776aaec363306f06009a591f, 0x173c7db824f1ff0df757193f2d2eab9cdfffe1d189f3d3a3e8ab0bf503dc70b8));
        vkX = pointAdd(vkX, scalarMul(G1Point(0x04f9fe21ad6233ad13f0a1c55e7eb4a20b30a186ee788dd09ee88cdf76eb7c89, 0x066b6bfd5c63393b9c1686032c71bec23d17c61cfe4ea82577d54fa125cd3f5d), input[0]));
        // __GAMMA_ABC_INLINE_END__

        return pairing(
            negate(proofA), proofB,
            vkX, GAMMA,
            proofC, DELTA,
            ALPHA, BETA
        );
    }

}
