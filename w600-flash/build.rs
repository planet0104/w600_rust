use std::env;
use std::path::{Path, PathBuf};

/// Windows 直接内置 SDK 附带的 wm_tool.exe；其他平台用本机的 C 编译器编译 SDK 提供的 wm_tool.c 后内置。
/// W800 的 wm_tool 依赖 zlib，使用随 SDK 提供的 zlib 源码。
fn main() {
    let tools = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../tools");
    println!("cargo:rerun-if-changed={}", tools.join("wm_tool").display());
    println!("cargo:rerun-if-changed={}", tools.join("wm_tool_w800").display());
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    compile(&[tools.join("wm_tool/wm_tool.c")], &out.join("wm_tool"));

    let w800 = tools.join("wm_tool_w800");
    let zlib = ["adler32.c", "crc32.c", "deflate.c", "gzclose.c", "gzlib.c", "gzwrite.c", "trees.c", "zutil.c"];
    let mut sources = vec![w800.join("wm_tool.c")];
    sources.extend(zlib.iter().map(|f| w800.join("zlib-1.2.11").join(f)));
    compile(&sources, &out.join("wm_tool_w800"));
}

fn compile(sources: &[PathBuf], out: &Path) {
    let status = cc::Build::new()
        .opt_level(2)
        .warnings(false)
        .get_compiler()
        .to_command()
        .args(sources)
        .arg("-o")
        .arg(out)
        .arg("-lpthread")
        .status()
        .expect("需要 C 编译器（gcc 或 clang）来编译 wm_tool");
    assert!(status.success(), "编译 {} 失败", out.display());
}
