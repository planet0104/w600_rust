use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    fs::copy(dir.join("w600.x"), out.join("w600.x")).unwrap();
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=native={}", dir.join("lib").display());
    println!("cargo:rerun-if-changed=w600.x");
    println!("cargo:rerun-if-changed=lib");
}
