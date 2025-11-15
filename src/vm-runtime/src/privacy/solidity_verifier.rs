// SPDX-License-Identifier: GPL-3.0-or-later
// Solidity Groth16 验证器生成工具
// 用于将 arkworks Groth16 验证密钥转换为 EVM 兼容的 Solidity 合约

// 默认导入 BLS12-381；支持 BN254 通过枚举后端切换
use ark_bls12_381::{Bls12_381, G1Affine as BlsG1Affine, G2Affine as BlsG2Affine};
use ark_bn254::{Bn254, G1Affine as BnG1Affine, G2Affine as BnG2Affine};
use ark_groth16::VerifyingKey;
use ark_serialize::CanonicalSerialize;
use std::io::Write;

/// Solidity 验证器生成器
pub enum CurveKind { BLS12_381, BN254 }

pub struct SolidityVerifierGenerator {
    contract_name: String,
    curve: CurveKind,
}

impl SolidityVerifierGenerator {
    /// 创建新的生成器
    pub fn new(contract_name: impl Into<String>) -> Self {
        Self { contract_name: contract_name.into(), curve: CurveKind::BLS12_381 }
    }

    pub fn with_curve(mut self, curve: CurveKind) -> Self {
        self.curve = curve;
        self
    }

    /// 生成 Solidity 验证合约代码
    /// 
    /// 参数:
    /// - vk: Groth16 验证密钥
    /// - num_public_inputs: 公共输入数量
    /// 
    /// 返回: Solidity 合约代码字符串
    pub fn generate_bls(&self, vk: &VerifyingKey<Bls12_381>, num_public_inputs: usize) -> String {
        let mut code = String::new();

        // SPDX 和 Pragma
        code.push_str("// SPDX-License-Identifier: MIT\n");
        code.push_str("pragma solidity ^0.8.0;\n\n");

        // 合约声明
        code.push_str(&format!("contract {} {{\n", self.contract_name));

        // Pairing 库结构体
        code.push_str("    // Pairing library structures\n");
        code.push_str("    struct G1Point {\n");
        code.push_str("        uint256 X;\n");
        code.push_str("        uint256 Y;\n");
        code.push_str("    }\n\n");

        code.push_str("    struct G2Point {\n");
        code.push_str("        uint256[2] X;\n");
        code.push_str("        uint256[2] Y;\n");
        code.push_str("    }\n\n");

        // 验证密钥常量
        code.push_str("    // Verification Key\n");
    code.push_str(&self.generate_vk_constants_bls(vk, num_public_inputs));

        // Pairing 检查函数
        code.push_str("    // Pairing check using precompile\n");
        code.push_str(&self.generate_pairing_function());

        // 验证函数
        code.push_str("    // Main verification function\n");
    code.push_str(&self.generate_verify_function_bls(vk, num_public_inputs));

        // 合约结束
        code.push_str("}\n");

        code
    }

    /// 生成 BN254 合约代码
    pub fn generate_bn254(&self, vk: &VerifyingKey<Bn254>, num_public_inputs: usize) -> String {
        let mut code = String::new();

        code.push_str("// SPDX-License-Identifier: MIT\n");
        code.push_str("pragma solidity ^0.8.0;\n\n");
        code.push_str(&format!("contract {} {{\n", self.contract_name));

        code.push_str("    // Pairing library structures\n");
        code.push_str("    struct G1Point {\n");
        code.push_str("        uint256 X;\n");
        code.push_str("        uint256 Y;\n");
        code.push_str("    }\n\n");

        code.push_str("    struct G2Point {\n");
        code.push_str("        uint256[2] X;\n");
        code.push_str("        uint256[2] Y;\n");
        code.push_str("    }\n\n");

        code.push_str("    // Verification Key\n");
        code.push_str(&self.generate_vk_constants_bn(vk, num_public_inputs));

        code.push_str("    // Pairing check using precompile\n");
        code.push_str(&self.generate_pairing_function());

        code.push_str("    // Main verification function\n");
        code.push_str(&self.generate_verify_function_bn(vk, num_public_inputs));

        code.push_str("}\n");
        code
    }

    /// 生成验证密钥常量定义
    fn generate_vk_constants_bls(&self, vk: &VerifyingKey<Bls12_381>, num_public_inputs: usize) -> String {
        let mut code = String::new();

        // Alpha (G1)
    let alpha = self.g1_to_solidity_bls(&vk.alpha_g1);
        code.push_str(&format!("    G1Point constant ALPHA = G1Point({}, {});\n", 
            alpha.0, alpha.1));

        // Beta (G2)
    let beta = self.g2_to_solidity_bls(&vk.beta_g2);
        code.push_str(&format!("    G2Point constant BETA = G2Point([{}, {}], [{}, {}]);\n",
            beta.0[0], beta.0[1], beta.1[0], beta.1[1]));

        // Gamma (G2)
    let gamma = self.g2_to_solidity_bls(&vk.gamma_g2);
        code.push_str(&format!("    G2Point constant GAMMA = G2Point([{}, {}], [{}, {}]);\n",
            gamma.0[0], gamma.0[1], gamma.1[0], gamma.1[1]));

        // Delta (G2)
    let delta = self.g2_to_solidity_bls(&vk.delta_g2);
        code.push_str(&format!("    G2Point constant DELTA = G2Point([{}, {}], [{}, {}]);\n",
            delta.0[0], delta.0[1], delta.1[0], delta.1[1]));

        // Gamma_ABC 不再生成动态数组函数，改为在 verifyProof 中内联展开以节省 gas
        code.push_str(&format!("\n    // Public inputs: {} (gamma_abc inline expansion in verifyProof)\n\n", num_public_inputs));

        code
    }

    fn generate_vk_constants_bn(&self, vk: &VerifyingKey<Bn254>, num_public_inputs: usize) -> String {
        let mut code = String::new();

        let alpha = self.g1_to_solidity_bn(&vk.alpha_g1);
        code.push_str(&format!("    G1Point constant ALPHA = G1Point({}, {});\n", alpha.0, alpha.1));

        let beta = self.g2_to_solidity_bn(&vk.beta_g2);
        code.push_str(&format!("    G2Point constant BETA = G2Point([{}, {}], [{}, {}]);\n", beta.0[0], beta.0[1], beta.1[0], beta.1[1]));

        let gamma = self.g2_to_solidity_bn(&vk.gamma_g2);
        code.push_str(&format!("    G2Point constant GAMMA = G2Point([{}, {}], [{}, {}]);\n", gamma.0[0], gamma.0[1], gamma.1[0], gamma.1[1]));

        let delta = self.g2_to_solidity_bn(&vk.delta_g2);
        code.push_str(&format!("    G2Point constant DELTA = G2Point([{}, {}], [{}, {}]);\n", delta.0[0], delta.0[1], delta.1[0], delta.1[1]));

        code.push_str(&format!("\n    // Public inputs: {} (gamma_abc inline expansion in verifyProof)\n\n", num_public_inputs));
        code
    }

    /// 生成 Pairing 检查函数
    fn generate_pairing_function(&self) -> String {
        let mut code = String::new();

        code.push_str("    function pairing(\n");
        code.push_str("        G1Point memory a1,\n");
        code.push_str("        G2Point memory a2,\n");
        code.push_str("        G1Point memory b1,\n");
        code.push_str("        G2Point memory b2,\n");
        code.push_str("        G1Point memory c1,\n");
        code.push_str("        G2Point memory c2,\n");
        code.push_str("        G1Point memory d1,\n");
        code.push_str("        G2Point memory d2\n");
        code.push_str("    ) internal view returns (bool) {\n");
        code.push_str("        G1Point[4] memory p1 = [a1, b1, c1, d1];\n");
        code.push_str("        G2Point[4] memory p2 = [a2, b2, c2, d2];\n");
        code.push_str("        uint256 inputSize = 24;\n");
        code.push_str("        uint256[] memory input = new uint256[](inputSize);\n\n");

        code.push_str("        for (uint256 i = 0; i < 4; i++) {\n");
        code.push_str("            uint256 j = i * 6;\n");
        code.push_str("            input[j + 0] = p1[i].X;\n");
        code.push_str("            input[j + 1] = p1[i].Y;\n");
        code.push_str("            input[j + 2] = p2[i].X[0];\n");
        code.push_str("            input[j + 3] = p2[i].X[1];\n");
        code.push_str("            input[j + 4] = p2[i].Y[0];\n");
        code.push_str("            input[j + 5] = p2[i].Y[1];\n");
        code.push_str("        }\n\n");

        code.push_str("        uint256[1] memory out;\n");
        code.push_str("        bool success;\n\n");

        code.push_str("        assembly {\n");
        code.push_str("            success := staticcall(\n");
        code.push_str("                sub(gas(), 2000),\n");
        code.push_str("                0x08,  // Precompile address for pairing check\n");
        code.push_str("                add(input, 0x20),\n");
        code.push_str("                mul(inputSize, 0x20),\n");
        code.push_str("                out,\n");
        code.push_str("                0x20\n");
        code.push_str("            )\n");
        code.push_str("        }\n\n");

        code.push_str("        require(success, \"Pairing check failed\");\n");
        code.push_str("        return out[0] != 0;\n");
        code.push_str("    }\n\n");

        // 添加辅助函数
        code.push_str("    // Negate a G1 point\n");
        code.push_str("    function negate(G1Point memory p) internal pure returns (G1Point memory) {\n");
        code.push_str("        uint256 q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;\n");
        code.push_str("        if (p.X == 0 && p.Y == 0) {\n");
        code.push_str("            return G1Point(0, 0);\n");
        code.push_str("        }\n");
        code.push_str("        return G1Point(p.X, q - (p.Y % q));\n");
        code.push_str("    }\n\n");

        code.push_str("    // Add two G1 points using precompile 0x06\n");
        code.push_str("    function pointAdd(G1Point memory p1, G1Point memory p2) internal view returns (G1Point memory) {\n");
        code.push_str("        uint256[4] memory input;\n");
        code.push_str("        input[0] = p1.X;\n");
        code.push_str("        input[1] = p1.Y;\n");
        code.push_str("        input[2] = p2.X;\n");
        code.push_str("        input[3] = p2.Y;\n\n");
        code.push_str("        bool success;\n");
        code.push_str("        G1Point memory result;\n");
        code.push_str("        assembly {\n");
        code.push_str("            success := staticcall(sub(gas(), 2000), 0x06, input, 0x80, result, 0x40)\n");
        code.push_str("        }\n");
        code.push_str("        require(success, \"Point addition failed\");\n");
        code.push_str("        return result;\n");
        code.push_str("    }\n\n");

        code.push_str("    // Scalar multiplication using precompile 0x07\n");
        code.push_str("    function scalarMul(G1Point memory p, uint256 s) internal view returns (G1Point memory) {\n");
        code.push_str("        uint256[3] memory input;\n");
        code.push_str("        input[0] = p.X;\n");
        code.push_str("        input[1] = p.Y;\n");
        code.push_str("        input[2] = s;\n\n");
        code.push_str("        bool success;\n");
        code.push_str("        G1Point memory result;\n");
        code.push_str("        assembly {\n");
        code.push_str("            success := staticcall(sub(gas(), 2000), 0x07, input, 0x60, result, 0x40)\n");
        code.push_str("        }\n");
        code.push_str("        require(success, \"Scalar multiplication failed\");\n");
        code.push_str("        return result;\n");
        code.push_str("    }\n\n");

        code
    }

    /// 生成主验证函数 (BLS12-381)
    fn generate_verify_function_bls(&self, vk: &VerifyingKey<Bls12_381>, num_public_inputs: usize) -> String {
        let mut code = String::new();

    code.push_str("    function verifyProof(\n");
    code.push_str("        uint256[2] calldata a,\n");
    code.push_str("        uint256[2][2] calldata b,\n");
    code.push_str("        uint256[2] calldata c,\n");
    code.push_str(&format!("        uint256[{}] calldata input\n", num_public_inputs));
    code.push_str("    ) external view returns (bool) {\n");
        code.push_str("        G1Point memory proofA = G1Point(a[0], a[1]);\n");
        code.push_str("        G2Point memory proofB = G2Point([b[0][0], b[0][1]], [b[1][0], b[1][1]]);\n");
        code.push_str("        G1Point memory proofC = G1Point(c[0], c[1]);\n\n");

        code.push_str("        // Compute vk_x = gamma_abc[0] + sum(input[i] * gamma_abc[i+1])\n");
        // 直接内联 gamma_abc，避免动态数组分配
        code.push_str("        G1Point memory vkX = G1Point(0,0);\n");
        code.push_str("        // vk_x = gamma_abc[0] + Σ input[i] * gamma_abc[i+1]\n");
        code.push_str("        // 以下点常量由生成器在编译期写入（Rust 侧内联）\n");
        // gamma_abc[0] + Σ input[i]*gamma_abc[i+1] 将在 save_to_file 时用占位符替换
        code.push_str("        // __GAMMA_ABC_INLINE_START__\n");
        // 我们在此插入占位符，稍后在 save_to_file 阶段进行替换
        code.push_str("        // __GAMMA_ABC_INLINE_END__\n\n");

        code.push_str("        // Pairing check: e(A, B) = e(alpha, beta) * e(vk_x, gamma) * e(C, delta)\n");
        code.push_str("        // Rearranged: e(A, B) * e(-vk_x, gamma) * e(-C, delta) = e(alpha, beta)\n");
        code.push_str("        return pairing(\n");
        code.push_str("            negate(proofA), proofB,\n");
        code.push_str("            vkX, GAMMA,\n");
        code.push_str("            proofC, DELTA,\n");
        code.push_str("            ALPHA, BETA\n");
        code.push_str("        );\n");
        code.push_str("    }\n\n");

        // 辅助函数: G1 点加法
        code.push_str("    // Helper: G1 point addition using precompile\n");
        code.push_str("    function pointAdd(G1Point memory p1, G1Point memory p2) internal view returns (G1Point memory) {\n");
        code.push_str("        uint256[4] memory input;\n");
        code.push_str("        input[0] = p1.X;\n");
        code.push_str("        input[1] = p1.Y;\n");
        code.push_str("        input[2] = p2.X;\n");
        code.push_str("        input[3] = p2.Y;\n");
        code.push_str("        bool success;\n");
        code.push_str("        G1Point memory result;\n");
        code.push_str("        assembly {\n");
        code.push_str("            success := staticcall(sub(gas(), 2000), 0x06, input, 0x80, result, 0x40)\n");
        code.push_str("        }\n");
        code.push_str("        require(success, \"Point addition failed\");\n");
        code.push_str("        return result;\n");
        code.push_str("    }\n\n");

        // 辅助函数: G1 标量乘法
        code.push_str("    // Helper: G1 scalar multiplication using precompile\n");
        code.push_str("    function scalarMul(G1Point memory p, uint256 s) internal view returns (G1Point memory) {\n");
        code.push_str("        uint256[3] memory input;\n");
        code.push_str("        input[0] = p.X;\n");
        code.push_str("        input[1] = p.Y;\n");
        code.push_str("        input[2] = s;\n");
        code.push_str("        bool success;\n");
        code.push_str("        G1Point memory result;\n");
        code.push_str("        assembly {\n");
        code.push_str("            success := staticcall(sub(gas(), 2000), 0x07, input, 0x60, result, 0x40)\n");
        code.push_str("        }\n");
        code.push_str("        require(success, \"Scalar multiplication failed\");\n");
        code.push_str("        return result;\n");
        code.push_str("    }\n\n");

        // 辅助函数: G1 点取反
        code.push_str("    // Helper: Negate G1 point\n");
        code.push_str("    function negate(G1Point memory p) internal pure returns (G1Point memory) {\n");
        code.push_str("        uint256 q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;\n");
        code.push_str("        if (p.X == 0 && p.Y == 0) return G1Point(0, 0);\n");
        code.push_str("        return G1Point(p.X, q - (p.Y % q));\n");
        code.push_str("    }\n");

        code
    }

    fn generate_verify_function_bn(&self, vk: &VerifyingKey<Bn254>, num_public_inputs: usize) -> String {
        let mut code = String::new();

        code.push_str("    function verifyProof(\n");
        code.push_str("        uint256[2] calldata a,\n");
        code.push_str("        uint256[2][2] calldata b,\n");
        code.push_str("        uint256[2] calldata c,\n");
        code.push_str(&format!("        uint256[{}] calldata input\n", num_public_inputs));
        code.push_str("    ) external view returns (bool) {\n");
        code.push_str("        G1Point memory proofA = G1Point(a[0], a[1]);\n");
        code.push_str("        G2Point memory proofB = G2Point([b[0][0], b[0][1]], [b[1][0], b[1][1]]);\n");
        code.push_str("        G1Point memory proofC = G1Point(c[0], c[1]);\n\n");

        code.push_str("        // Compute vk_x = gamma_abc[0] + sum(input[i] * gamma_abc[i+1])\n");
        code.push_str("        G1Point memory vkX = G1Point(0,0);\n");
        code.push_str("        // __GAMMA_ABC_INLINE_START__\n");
        code.push_str("        // __GAMMA_ABC_INLINE_END__\n\n");

        code.push_str("        return pairing(\n");
        code.push_str("            negate(proofA), proofB,\n");
        code.push_str("            vkX, GAMMA,\n");
        code.push_str("            proofC, DELTA,\n");
        code.push_str("            ALPHA, BETA\n");
        code.push_str("        );\n");
        code.push_str("    }\n\n");

        // 同 bls，公用 pairing/pointAdd/scalarMul/negate 实现（q 为 bn254 素数，已与预编译一致）
        code
    }

    /// 将 BLS12-381 G1 点转换为 Solidity 格式 (X, Y)
    fn g1_to_solidity_bls(&self, point: &BlsG1Affine) -> (String, String) {
        // 使用 x() 和 y() 方法分别获取坐标
        let x = point.x;
        let y = point.y;
        
        let mut x_bytes = Vec::new();
        x.serialize_uncompressed(&mut x_bytes).unwrap();
        let mut y_bytes = Vec::new();
        y.serialize_uncompressed(&mut y_bytes).unwrap();

        // 转换为大端序的十六进制字符串
        let x_hex = self.bytes_to_uint256(&x_bytes);
        let y_hex = self.bytes_to_uint256(&y_bytes);

        (x_hex, y_hex)
    }

    /// 将 BLS12-381 G2 点转换为 Solidity 格式 ([X0, X1], [Y0, Y1])
    fn g2_to_solidity_bls(&self, point: &BlsG2Affine) -> ([String; 2], [String; 2]) {
        // 使用 x 和 y 字段直接访问坐标
        let x = &point.x;
        let y = &point.y;
        
        // G2 点的坐标是 Fq2,包含两个 Fq 分量 (c0, c1)
        let mut x_c0_bytes = Vec::new();
        let mut x_c1_bytes = Vec::new();
        x.c0.serialize_uncompressed(&mut x_c0_bytes).unwrap();
        x.c1.serialize_uncompressed(&mut x_c1_bytes).unwrap();

        let mut y_c0_bytes = Vec::new();
        let mut y_c1_bytes = Vec::new();
        y.c0.serialize_uncompressed(&mut y_c0_bytes).unwrap();
        y.c1.serialize_uncompressed(&mut y_c1_bytes).unwrap();

        let x_array = [
            self.bytes_to_uint256(&x_c0_bytes),
            self.bytes_to_uint256(&x_c1_bytes),
        ];
        let y_array = [
            self.bytes_to_uint256(&y_c0_bytes),
            self.bytes_to_uint256(&y_c1_bytes),
        ];

        (x_array, y_array)
    }

    /// 将 BN254 G1 点转换为 Solidity 格式
    fn g1_to_solidity_bn(&self, point: &BnG1Affine) -> (String, String) {
        let x = point.x;
        let y = point.y;
        let mut x_bytes = Vec::new();
        x.serialize_uncompressed(&mut x_bytes).unwrap();
        let mut y_bytes = Vec::new();
        y.serialize_uncompressed(&mut y_bytes).unwrap();
        (self.bytes_to_uint256(&x_bytes), self.bytes_to_uint256(&y_bytes))
    }

    /// 将 BN254 G2 点转换为 Solidity 格式
    fn g2_to_solidity_bn(&self, point: &BnG2Affine) -> ([String; 2], [String; 2]) {
        let x = &point.x;
        let y = &point.y;
        let mut x_c0_bytes = Vec::new();
        let mut x_c1_bytes = Vec::new();
        x.c0.serialize_uncompressed(&mut x_c0_bytes).unwrap();
        x.c1.serialize_uncompressed(&mut x_c1_bytes).unwrap();
        let mut y_c0_bytes = Vec::new();
        let mut y_c1_bytes = Vec::new();
        y.c0.serialize_uncompressed(&mut y_c0_bytes).unwrap();
        y.c1.serialize_uncompressed(&mut y_c1_bytes).unwrap();
        (
            [self.bytes_to_uint256(&x_c0_bytes), self.bytes_to_uint256(&x_c1_bytes)],
            [self.bytes_to_uint256(&y_c0_bytes), self.bytes_to_uint256(&y_c1_bytes)],
        )
    }

    /// 将字节数组转换为 uint256 字符串
    fn bytes_to_uint256(&self, bytes: &[u8]) -> String {
        // arkworks 使用小端序,需要转换为大端序
        let mut reversed = bytes.to_vec();
        reversed.reverse();
        
        // 转换为十六进制字符串
        let hex = reversed.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        
        format!("0x{}", hex)
    }

    /// 将生成的合约保存到文件
    pub fn save_to_file(&self, vk: &VerifyingKey<Bls12_381>, num_public_inputs: usize, path: &str) -> std::io::Result<()> {
        // 先生成基础代码 (BLS12-381)
        let mut code = self.generate_bls(vk, num_public_inputs);

        // 在占位符处内联展开 gamma_abc 累加表达式，减少 runtime 分配
        let mut inline = String::new();
        // gamma_abc[0]
    let g0 = self.g1_to_solidity_bls(&vk.gamma_abc_g1[0]);
        inline.push_str(&format!("        vkX = pointAdd(vkX, G1Point({}, {}));\n", g0.0, g0.1));
        // Σ input[i] * gamma_abc[i+1]
        for i in 0..num_public_inputs {
            let gp = self.g1_to_solidity_bls(&vk.gamma_abc_g1[i + 1]);
            inline.push_str(&format!(
                "        vkX = pointAdd(vkX, scalarMul(G1Point({}, {}), input[{}]));\n",
                gp.0, gp.1, i
            ));
        }

        code = code.replace(
            "        // __GAMMA_ABC_INLINE_START__\n        // __GAMMA_ABC_INLINE_END__\n\n",
            &format!("        // __GAMMA_ABC_INLINE_START__\n{}        // __GAMMA_ABC_INLINE_END__\n\n", inline),
        );

        let mut file = std::fs::File::create(path)?;
        file.write_all(code.as_bytes())?;
        Ok(())
    }

    /// 保存 BN254 合约到文件，并将 gamma_abc 进行内联展开
    pub fn save_to_file_bn(&self, vk: &VerifyingKey<Bn254>, num_public_inputs: usize, path: &str) -> std::io::Result<()> {
        let mut code = self.generate_bn254(vk, num_public_inputs);

        let mut inline = String::new();
        let g0 = self.g1_to_solidity_bn(&vk.gamma_abc_g1[0]);
        inline.push_str(&format!("        vkX = pointAdd(vkX, G1Point({}, {}));\n", g0.0, g0.1));
        for i in 0..num_public_inputs {
            let gp = self.g1_to_solidity_bn(&vk.gamma_abc_g1[i + 1]);
            inline.push_str(&format!(
                "        vkX = pointAdd(vkX, scalarMul(G1Point({}, {}), input[{}]));\n",
                gp.0, gp.1, i
            ));
        }

        code = code.replace(
            "        // __GAMMA_ABC_INLINE_START__\n        // __GAMMA_ABC_INLINE_END__\n\n",
            &format!("        // __GAMMA_ABC_INLINE_START__\n{}        // __GAMMA_ABC_INLINE_END__\n\n", inline),
        );

        let mut file = std::fs::File::create(path)?;
        file.write_all(code.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::UniformRand;
    use zk_groth16_test::MultiplyCircuit;
    use ark_groth16::Groth16;
    use ark_snark::SNARK;

    #[test]
    fn test_generate_solidity_verifier() {
        let mut rng = rand::rngs::OsRng;
        
        // 生成测试电路的 setup
        let a = ark_bls12_381::Fr::rand(&mut rng);
        let b = ark_bls12_381::Fr::rand(&mut rng);
        let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
        
        let (_pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng)
            .expect("Setup failed");

        // 生成 Solidity 代码
    let generator = SolidityVerifierGenerator::new("MultiplyVerifier");
    let code = generator.generate_bls(&vk, 1); // MultiplyCircuit 有 1 个公共输入 (c = a*b)

        // 验证生成的代码包含关键部分
        assert!(code.contains("contract MultiplyVerifier"));
        assert!(code.contains("function verifyProof"));
        assert!(code.contains("G1Point"));
        assert!(code.contains("G2Point"));
        assert!(code.contains("ALPHA"));
        assert!(code.contains("BETA"));
        assert!(code.contains("GAMMA"));
        assert!(code.contains("DELTA"));

        println!("Generated Solidity verifier ({} bytes):", code.len());
        println!("{}", &code[..500.min(code.len())]);
    }

    #[test]
    fn test_save_solidity_verifier() {
        let mut rng = rand::rngs::OsRng;
        
        let a = ark_bls12_381::Fr::rand(&mut rng);
        let b = ark_bls12_381::Fr::rand(&mut rng);
        let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
        
        let (_pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng)
            .expect("Setup failed");

        // 创建临时目录
        std::fs::create_dir_all("target/contracts")
            .expect("Failed to create directory");
        
        // 保存到文件
        let temp_path = "target/contracts/MultiplyVerifier.sol";
        let generator = SolidityVerifierGenerator::new("MultiplyVerifier");
        generator.save_to_file(&vk, 1, temp_path)
            .expect("Failed to save file");

        // 验证文件存在
        assert!(std::path::Path::new(temp_path).exists());
        
        // 读取并验证内容
        let content = std::fs::read_to_string(temp_path)
            .expect("Failed to read file");
        assert!(content.contains("pragma solidity"));

        println!("Saved verifier to: {}", temp_path);
    }
}
