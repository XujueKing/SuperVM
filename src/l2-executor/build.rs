#[cfg(all(feature = "risc0-poc", not(windows)))]
fn main() {
    // Allow skipping RISC0 build if toolchain is not available
    // Set RISC0_SKIP_BUILD=1 to bypass risc0_build::embed_methods()
    if std::env::var("RISC0_SKIP_BUILD").is_ok() {
        println!("cargo:warning=RISC0_SKIP_BUILD is set, skipping risc0_build::embed_methods()");
        return;
    }

    // Check if risc0 toolchain is available before attempting to build
    let rzup_check = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v cargo-risczero")
        .output();

    if let Ok(output) = rzup_check {
        if !output.status.success() {
            println!("cargo:warning=RISC0 toolchain not found. Set RISC0_SKIP_BUILD=1 to bypass, or install via: curl -L https://risczero.com/install | bash && rzup install");
            // Only fail if we're not in a dev/local environment
            if std::env::var("CI").is_err() {
                println!("cargo:warning=Skipping RISC0 build in local environment without toolchain");
                return;
            }
        }
    }

    risc0_build::embed_methods();
}

#[cfg(any(not(feature = "risc0-poc"), windows))]
fn main() {
    // RISC0 only supported on Unix-like systems (Linux/macOS)
    // On Windows, build script is a no-op
}
