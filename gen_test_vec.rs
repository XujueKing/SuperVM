// 临时程序生成测试向量
use k256::ecdsa::SigningKey;
use sha3::{Keccak256, Digest};

fn main() {
    let sk_bytes = [1u8; 32];
    let sk = SigningKey::from_bytes((&sk_bytes).into()).unwrap();
    let vk = sk.verifying_key();
    let pubkey_uncompressed = vk.to_encoded_point(false);
    let pubkey_bytes = pubkey_uncompressed.as_bytes();
    
    println!("公钥 (65 字节未压缩):");
    print!("\"");
    for (i, b) in pubkey_bytes.iter().enumerate() {
        print!("\\{:02x}", b);
        if (i + 1) % 16 == 0 && i != pubkey_bytes.len() - 1 {
            print!("\"\n      \"");
        }
    }
    println!("\"");
    
    // 计算地址
    let hash = Keccak256::digest(&pubkey_bytes[1..]);
    let addr = &hash[12..];
    println!("\n派生的地址:");
    print!("0x");
    for b in addr {
        print!("{:02x}", b);
    }
    println!();
}
