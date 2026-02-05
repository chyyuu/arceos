use std::path::PathBuf;

fn main() {
    // Get the profile (debug or release)
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
    let target = std::env::var("TARGET").unwrap_or_default();
    
    // Check if building via Makefile by examining AX_CONFIG_PATH
    // When using Makefile: AX_CONFIG_PATH points to root .axconfig.toml
    // When using cargo run: AX_CONFIG_PATH points to local axconfig.toml
    let ax_config_path = std::env::var("AX_CONFIG_PATH").unwrap_or_default();
    let is_makefile_build = ax_config_path.contains(".axconfig.toml") 
        && !ax_config_path.contains("examples/helloworld");
    
    // Only set linker script for bare metal targets and when NOT using Makefile
    // (Makefile sets linker args via RUSTFLAGS)
    if target.contains("none") && !is_makefile_build {
        // Get the workspace root directory
        // CARGO_MANIFEST_DIR is the directory containing Cargo.toml (examples/helloworld)
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let workspace_root = PathBuf::from(&manifest_dir)
            .parent() // examples
            .and_then(|p| p.parent()) // arceos root
            .expect("Failed to find workspace root")
            .to_path_buf();
        
        // Construct the linker script path
        let linker_script = workspace_root
            .join("target")
            .join(&target)
            .join(&profile)
            .join("linker_riscv64-qemu-virt.lds");
        
        // Tell cargo to pass the linker script to rustc
        println!("cargo:rustc-link-arg=-T{}", linker_script.display());
        println!("cargo:rustc-link-arg=-no-pie");
        println!("cargo:rustc-link-arg=-znostart-stop-gc");
        
        // Rerun if the linker script changes
        println!("cargo:rerun-if-changed={}", linker_script.display());
    }
    
    // Rerun if build profile or config path changes
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=AX_CONFIG_PATH");
}
