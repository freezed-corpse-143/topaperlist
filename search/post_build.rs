use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("CRATE_OUT_DIR").expect("CRATE_OUT_DIR not set (run via cargo post build)");
    let manifest_dir = env::var("CRATE_MANIFEST_DIR").expect("CRATE_MANIFEST_DIR not set");

    let binary_name = if cfg!(target_os = "windows") {
        "search.exe"
    } else {
        "search"
    };

    let src = Path::new(&out_dir).join(binary_name);
    let dest = Path::new(&manifest_dir).join("..").join(binary_name);

    fs::copy(&src, &dest).unwrap_or_else(|e| {
        panic!(
            "Failed to copy binary from {} to {}: {e}",
            src.display(),
            dest.display()
        );
    });

    println!(
        "cargo-post: copied {} -> {}",
        binary_name,
        dest.canonicalize().unwrap_or_else(|_| dest.clone()).display()
    );
}
