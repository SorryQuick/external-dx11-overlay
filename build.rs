use std::env;
use std::fs;
use std::path::PathBuf;

// Helper: returns true if the given feature is enabled
fn feature_enabled(name: &str) -> bool {
    env::var(format!("CARGO_FEATURE_{}", name.to_uppercase())).is_ok()
}

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR missing");
    let target_dir = PathBuf::from(out_dir)
        .ancestors()
        .nth(3) // Go up to target/{debug|release}
        .expect("Couldn't get target dir")
        .to_path_buf();

    let lib_name = "external_dx11_overlay";
    let dll_name = format!("{}.dll", lib_name);
    let src_path = target_dir.join(&dll_name);

    // rename dll to nexus_blishud_overlay_loader to differentiate the 2
    if feature_enabled("nexus") {
        let new_name = format!("nexus_blishhud_overlay_loader.dll");
        let dst_path = target_dir.join(&new_name);

        // Try to copy (or rename) output DLL
        if src_path.exists() {
            // Rename or copy
            fs::copy(&src_path, &dst_path)
                .expect("Failed to copy DLL file");
            println!("cargo:warning=DLL renamed to {}", new_name);
        } else {
            println!("cargo:warning=DLL not found at expected path");
        }
    }
}
