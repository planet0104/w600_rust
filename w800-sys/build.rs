use std::env;
use std::fs;
use std::path::PathBuf;

/// 与官方 Makefile 相同，SDK 库整体链接（--whole-archive）：中断处理函数等以强符号覆盖启动文件中的弱符号，
/// 没有其他地方引用它们，按需链接会把它们漏掉。未使用的代码仍会被 --gc-sections 去掉。
const LIBS: &[&str] = &[
    "wmarch", "wmcommon", "drivers", "network", "os", "wmsys", "blehost", "app", "wlan", "dsp", "btcontroller",
];

fn main() {
    let dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    fs::copy(dir.join("w800.x"), out.join("w800.x")).unwrap();
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-search=native={}", dir.join("lib").display());
    for lib in LIBS {
        println!("cargo:rustc-link-lib=static:+whole-archive,-bundle={lib}");
    }
    println!("cargo:rerun-if-changed=w800.x");
    println!("cargo:rerun-if-changed=lib");
}
