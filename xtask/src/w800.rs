//! W800（W80X_SDK_v1.00.10）：SDK 自带各模块的预编译库，直接复制（去掉调试信息）即可，不需要重新编译。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{download_verified, run, workspace_root, Result, FUNCTIONS, TYPES, VARS};

const SDK_URL: &str = "https://www.winnermicro.com/upload/1/editor/1766456383634.rar";
const SDK_SHA256: &str = "4236d60e473babd406d0656677055e617d5167386963249bc027c861031a407f";
const SDK_DIR_NAME: &str = "W80X_SDK_v1.00.10";

/// 官方默认配置（tools/w800/.config：USE_LIB=y、USE_NIMBLE=y）链接的全部库，见 SDK 根目录的 Makefile。
const LIBS: &[&str] = &[
    "wmarch", "wmcommon", "drivers", "network", "os", "wmsys", "blehost", "app", "wlan", "dsp", "btcontroller",
];

/// wm_tool.c 依赖的 zlib 源文件（Linux / macOS 上由 w600-flash 的 build.rs 编译）。
const ZLIB_FILES: &[&str] = &[
    "adler32.c", "crc32.c", "crc32.h", "deflate.c", "deflate.h", "gzclose.c", "gzguts.h", "gzlib.c", "gzwrite.c",
    "trees.c", "trees.h", "zconf.h", "zlib.h", "zutil.c", "zutil.h",
];

/// 与 tools/w800/inc.mk 一致，不存在的目录会被自动跳过。
const INCLUDE_DIRS: &[&str] = &[
    "include",
    "include/app",
    "include/arch/xt804",
    "include/arch/xt804/csi_core",
    "include/arch/xt804/csi_dsp",
    "include/driver",
    "include/net",
    "include/os",
    "include/platform",
    "include/wifi",
    "include/bt",
    "platform/common/params",
    "platform/inc",
    "platform/sys",
    "src/app",
    "src/app/wm_atcmd",
    "src/app/dhcpserver",
    "src/app/dnsserver",
    "src/app/web",
    "src/app/cloud",
    "src/app/cJSON",
    "src/app/ntp",
    "src/app/httpclient",
    "src/app/oneshotconfig",
    "src/app/iperf",
    "src/app/mqtt",
    "src/app/ping",
    "src/app/polarssl/include",
    "src/app/ota",
    "src/app/mbedtls/include",
    "src/app/mbedtls/ports",
    "src/network/api_wm",
    "src/network/lwip2.1.3/include",
    "src/os/rtos/include",
    "demo",
];

/// 与 tools/w800/conf.mk 中的 CCFLAGS 一致。
const DEFINES: &[&str] = &[
    "TLS_CONFIG_CPU_XT804=1",
    "GCC_COMPILE=1",
    "NO_PERSISTENCE=1",
    "USE_SELECT=1",
    "HIGH_PERFORMANCE=1",
    "USE_MBEDTLS=1",
    "NIMBLE_FTR=1",
];

fn work_dir() -> PathBuf {
    workspace_root().join("target").join("w800-sdk")
}

fn tool(name: &str) -> PathBuf {
    let exe = format!("csky-elfabiv2-{name}{}", env::consts::EXE_SUFFIX);
    match env::var_os("W800_TOOLCHAIN") {
        Some(dir) => PathBuf::from(dir).join(exe),
        None => PathBuf::from(exe),
    }
}

/// 返回 SDK 根目录，必要时下载、校验并解压。官方只提供 RAR 包，用 bsdtar（libarchive）解压，
/// Windows 10 及以上自带的 tar 就是 bsdtar。
fn ensure_sdk() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("W800_SDK_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let work = work_dir();
    let sdk = work.join(SDK_DIR_NAME);
    if sdk.join("include").is_dir() {
        return Ok(sdk);
    }
    fs::create_dir_all(&work)?;
    let rar = work.join("sdk.rar");
    download_verified(SDK_URL, SDK_SHA256, &rar)?;
    println!("解压 SDK 到 {}", sdk.display());
    let tar = if cfg!(windows) { "tar" } else { "bsdtar" };
    run(Command::new(tar).arg("-xf").arg(&rar).arg("-C").arg(&work))
        .map_err(|e| format!("{e}\n解压 RAR 需要 bsdtar（Debian/Ubuntu: apt install libarchive-tools）"))?;
    Ok(sdk)
}

pub fn build_sdk() -> Result<()> {
    let sdk = ensure_sdk()?;
    let root = workspace_root();

    let lib_dir = root.join("w800-sys").join("lib");
    fs::create_dir_all(&lib_dir)?;
    for name in LIBS {
        let file = format!("lib{name}.a");
        let out = lib_dir.join(&file);
        // 官方库带有完整调试信息（共约 30MB），去掉后约 5MB；-D 保证重复执行得到相同的文件。
        run(Command::new(tool("strip"))
            .args(["--strip-debug", "-D"])
            .arg(sdk.join("lib/w800").join(&file))
            .arg("-o")
            .arg(&out))?;
        println!("{}", out.display());
    }

    let bin_dir = root.join("w800-sys").join("bin");
    fs::create_dir_all(&bin_dir)?;
    fs::copy(sdk.join("tools/w800/w800_secboot.img"), bin_dir.join("secboot.img"))?;

    let tool_dir = root.join("tools").join("wm_tool_w800");
    let zlib_dir = tool_dir.join("zlib-1.2.11");
    fs::create_dir_all(&zlib_dir)?;
    fs::copy(sdk.join("tools/w800/wm_tool.exe"), tool_dir.join("wm_tool.exe"))?;
    fs::copy(sdk.join("tools/w800/wm_tool.c"), tool_dir.join("wm_tool.c"))?;
    for name in ZLIB_FILES {
        fs::copy(sdk.join("tools/w800/zlib-1.2.11").join(name), zlib_dir.join(name))?;
    }

    println!("完成: {}", lib_dir.display());
    Ok(())
}

fn include_args(sdk: &Path) -> Vec<String> {
    INCLUDE_DIRS
        .iter()
        .map(|d| sdk.join(d))
        .filter(|d| d.is_dir())
        .map(|d| format!("-I{}", d.display()))
        .chain(DEFINES.iter().map(|d| format!("-D{d}")))
        .collect()
}

pub fn generate_bindings() -> Result<()> {
    let sdk = ensure_sdk()?;
    let root = workspace_root();
    let gcc = tool("gcc");
    let gcc_inc = PathBuf::from(run(Command::new(&gcc).arg("-print-file-name=include"))?);
    // minilibc 的头文件位于 <prefix>/csky-elfabiv2/include，即 lib/gcc/csky-elfabiv2/<版本> 往上四级。
    let libc_inc = gcc_inc.join("../../../../../csky-elfabiv2/include");

    let mut clang_args: Vec<String> = [
        "--target=csky-unknown-elf",
        "-mfloat-abi=hard",
        // C-SKY 的 char 是无符号的（GCC 预定义了 __CHAR_UNSIGNED__）。
        "-funsigned-char",
        "-std=gnu99",
        "-nostdinc",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // minilibc 的 stddef.h 等会 #include_next 编译器自带的同名头文件，所以必须排在前面。
    for dir in [libc_inc, gcc_inc.clone(), gcc_inc.with_file_name("include-fixed")] {
        clang_args.push(format!("-isystem{}", dir.display()));
    }
    clang_args.extend(include_args(&sdk));

    let bindings = bindgen::Builder::default()
        .header(root.join("w800-sys/wrapper.h").display().to_string())
        .clang_args(&clang_args)
        .use_core()
        .ctypes_prefix("core::ffi")
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .derive_default(true)
        .generate_comments(false)
        .merge_extern_blocks(true)
        .allowlist_function(format!("{FUNCTIONS}|sendchar|close|lwip_gethostbyname"))
        .allowlist_type(TYPES)
        .allowlist_var(format!("{VARS}|LWIP_SOCKET_OFFSET"))
        .generate()
        .map_err(|e| format!("bindgen 失败: {e}"))?;

    let out = root.join("w800-sys/src/bindings.rs");
    let header = format!("// 由 `cargo xtask w800-bindgen` 从 {SDK_DIR_NAME} 头文件生成，请勿手动修改。\n\n");
    fs::write(&out, format!("{header}{bindings}"))?;
    println!("完成: {}", out.display());
    Ok(())
}
