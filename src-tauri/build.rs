fn main() {
    // Pass the target triple to the code so sidecar lookup can use it
    println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());
    tauri_build::build();
}
