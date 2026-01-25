fn main() {
    if let Ok(os) = std::env::var("CARGO_CFG_TARGET_OS") {
        if os == "windows" {
            println!("cargo:rustc-link-lib=./assets/res");
        }
    }
}
