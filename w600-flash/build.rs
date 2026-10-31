use std::env;
use std::path::PathBuf;

/// Windows 直接内置 SDK 附带的 wm_tool.exe；其他平台用本机的 C 编译器编译 SDK 提供的 wm_tool.c 后内置。
fn main() {
    let src = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../tools/wm_tool/wm_tool.c");
    println!("cargo:rerun-if-changed={}", src.display());
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("wm_tool");
    let status = cc::Build::new()
        .opt_level(2)
        .warnings(false)
        .get_compiler()
        .to_command()
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .arg("-lpthread")
        .status()
        .expect("需要 C 编译器（gcc 或 clang）来编译 wm_tool");
    assert!(status.success(), "编译 wm_tool 失败");
}
