#[cfg(all(feature = "risc0-poc", not(windows)))]
fn main() {
    // Soft-fail strategy: only embed RISC0 guest methods if toolchain is really present
    // Control env vars:
    //   RISC0_SKIP_BUILD=1       -> unconditional skip
    //   RISC0_FORCE_EMBED=1      -> force embed even if detection fails (debug)
    //   CI                       -> do NOT change behavior, still soft skip if missing

    if std::env::var("RISC0_SKIP_BUILD").is_ok() {
        println!("cargo:warning=RISC0_SKIP_BUILD is set, skipping risc0_build::embed_methods()");
        return;
    }

    let force = std::env::var("RISC0_FORCE_EMBED").is_ok();

    // Multi-path detection: cargo-risczero in PATH and presence of ~/.risc0 directory
    let has_cargo_risczero = std::process::Command::new("sh")
        .arg("-c")
        .arg("command -v cargo-risczero")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let risczero_dir = std::env::var("HOME")
        .map(|h| std::path::Path::new(&h).join(".risc0"))
        .unwrap_or_else(|_| std::path::PathBuf::from("/.risc0"));
    let has_risc0_dir = risczero_dir.exists();

    if !has_cargo_risczero || !has_risc0_dir {
        if !force {
            println!("cargo:warning=RISC0 toolchain not fully detected (cargo-risczero={}, .risc0 dir={}). Skipping guest embedding.", has_cargo_risczero, has_risc0_dir);
            println!("cargo:warning=To force embedding set RISC0_FORCE_EMBED=1. To skip explicitly set RISC0_SKIP_BUILD=1.");
            println!("cargo:warning=Install instructions: curl -L https://risczero.com/install | bash && rzup install");
            return;
        } else {
            println!("cargo:warning=RISC0_FORCE_EMBED set; proceeding despite missing toolchain detection (cargo-risczero={}, .risc0 dir={}).", has_cargo_risczero, has_risc0_dir);
        }
    }

    // Perform embedding; any failure inside risc0_build will become a hard error
    risc0_build::embed_methods();
}

#[cfg(any(not(feature = "risc0-poc"), windows))]
fn main() {
    // RISC0 only supported on Unix-like systems (Linux/macOS)
    // On Windows, build script is a no-op
}
