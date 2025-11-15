fn main() {
    // 仅在启用 cross-shard feature 时进行编译，避免无 protoc 情况构建失败
    let features = std::env::var("CARGO_FEATURE_CROSS_SHARD").is_ok();
    if !features {
        return;
    }

    let proto_path = std::path::Path::new("../../proto/cross_shard.proto");
    if !proto_path.exists() {
        println!("cargo:warning=proto file not found: {}", proto_path.display());
        return;
    }
    println!("cargo:rerun-if-changed={}", proto_path.display());
    println!("cargo:rerun-if-changed=../../proto");

    // 使用 vendored protoc，避免本地缺失
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc not found");
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir(std::env::var("OUT_DIR").unwrap())
        .compile(&[proto_path.to_str().unwrap()], &["../../proto"])
        .expect("Failed to compile gRPC protos");
}
