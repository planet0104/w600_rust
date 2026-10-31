//! 维护者工具：下载官方 SDK，生成 `w600-sys` / `w800-sys` 使用的预编译库和 Rust 绑定。
//!
//! 用法：
//!   cargo xtask sdk            下载 W600 SDK 并编译出 w600-sys/lib/*.a
//!   cargo xtask bindgen        从 W600 SDK 头文件生成 w600-sys/src/bindings.rs
//!   cargo xtask all            依次执行以上两步
//!   cargo xtask w800-sdk       下载 W800 SDK，复制其预编译库到 w800-sys/lib
//!   cargo xtask w800-bindgen   从 W800 SDK 头文件生成 w800-sys/src/bindings.rs
//!   cargo xtask w800-all       依次执行以上两步
//!
//! 环境变量：
//!   W600_SDK_DIR    使用已解压（或自行修改过）的 SDK 目录，而不是自动下载
//!   W600_TOOLCHAIN  arm-none-eabi 工具链的 bin 目录，未设置时从 PATH 查找
//!   W800_SDK_DIR    同 W600_SDK_DIR，用于 W800
//!   W800_TOOLCHAIN  csky-elfabiv2 工具链的 bin 目录，未设置时从 PATH 查找

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;

use sha2::{Digest, Sha256};

mod w800;

const SDK_URL: &str = "https://www.winnermicro.com/upload/1/editor/1584693446529.zip";
const SDK_SHA256: &str = "2ec93cafc70176592ba84bc2ea3ebac9d517c73b2f112f525420660b016c6a6e";
const SDK_DIR_NAME: &str = "WM_SDK_W60X_G3.04.00";

/// 编进 libw600sdk.a 的模块目录，对应官方 Tools/GNU/Makefile 的链接列表（去掉了 demo/iperf 等示例）。
const MODULES: &[&str] = &[
    "Platform/Boot/gcc",
    "Platform/Sys",
    "Platform/Common",
    "Platform/Drivers",
    "Src/OS/RTOS",
    "Src/Network",
    "Src/App/wm_atcmd",
    "Src/App/oneshotconfig",
    "Src/App/dhcpserver",
    "Src/App/dnsserver",
    "Src/App/httpclient",
    "Src/App/OTA",
    "Src/App/ntp",
    "Src/App/ping",
    "Src/App/matrixssl",
    "Src/App/web",
    "Src/App/libwebsockets-2.1-stable",
    "Src/App/sslserver",
    "Src/App/cJSON",
];

/// 与 Tools/toolchain.def 中的 INCLUDES 一致，不存在的目录会被自动跳过。
const INCLUDE_DIRS: &[&str] = &[
    "Include",
    "Include/App",
    "Include/Net",
    "Include/WiFi",
    "Include/OS",
    "Include/Driver",
    "Include/Platform",
    "Src/OS/RTOS/include",
    "Platform/Common/Params",
    "Platform/Common/task",
    "Platform/Common/mem",
    "Platform/Common/fwup",
    "Platform/Common/utils",
    "Platform/Common/crypto",
    "Platform/Common/crypto/symmetric",
    "Platform/Common/crypto/digest",
    "Platform/Common/crypto/math",
    "Platform/Inc",
    "Platform/Sys",
    "Platform/Boot/gcc",
    "Src/App/wm_atcmd",
    "Src/App/matrixssl",
    "Src/App/matrixssl/core",
    "Src/App/web",
    "Src/App/OTA",
    "Src/App/cloud",
    "Src/App/cJSON",
    "Src/App/oneshotconfig",
    "Src/App/dhcpserver",
    "Src/App/dnsserver",
    "Src/App/ping",
    "Src/App/iperf",
    "Src/App/libcoap/include",
    "Src/App/polarssl/include",
    "Src/App/mDNS/mDNSCore",
    "Src/App/mDNS/mDNSPosix",
    "Src/App/mqtt",
    "Src/App/easylogger/inc",
    "Src/App/libwebsockets-2.1-stable",
    "Src/App/httpclient",
    "Demo",
    "Src/Network/api2.0.3",
    "Src/Network/lwip2.0.3",
    "Src/Network/lwip2.0.3/include",
    "Src/Network/lwip2.0.3/include/arch",
    "Src/Network/lwip2.0.3/include/lwip",
    "Src/Network/lwip2.0.3/include/netif",
];

const DEFINES: &[&str] = &[
    "GCC_COMPILE=1",
    "TLS_COST_DOWN=0",
    "_IN_ADDR_T_DECLARED",
    "__MACHINE_ENDIAN_H__",
    "_TIMEVAL_DEFINED",
    "__INSIDE_CYGWIN_NET__",
    // fd_set/select 由 lwIP 提供，阻止新版 newlib 的 sys/select.h 重复定义。
    "__USE_W32_SOCKETS",
    // 启动文件不再预留固定大小的 .heap，堆范围由 w600.x 决定（从 end 到 RAM 末尾）。
    "__HEAP_SIZE=0",
];

/// 对 Include/wm_config.h 的修改。关闭 AT 指令子系统（及其依赖的网页配网、SSL 等），
/// 可省下约 150KB Flash，并把 UART0 让给应用。这些开关不影响任何与 wlan.a 共享的结构体。
const CONFIG_OVERRIDES: &[(&str, &str)] = &[("TLS_CONFIG_HOSTIF", "CFG_OFF")];

const CPU_FLAGS: &[&str] = &["-mcpu=cortex-m3", "-mthumb", "-mabi=aapcs"];

/// 这份 SDK 写于 GCC 4.x 时代；新版 GCC 把隐式声明、指针类型不匹配等改成了错误，
/// `-fpermissive` 把它们降回警告，`-fcommon` 恢复旧的公共符号合并行为。
const C_FLAGS: &[&str] = &[
    "-O1",
    "-std=gnu99",
    "-ffunction-sections",
    "-fdata-sections",
    "-fno-builtin",
    "-fcommon",
    "-fpermissive",
    "-w",
];

fn main() {
    let cmd = env::args().nth(1).unwrap_or_default();
    let result = match cmd.as_str() {
        "sdk" => build_sdk(),
        "bindgen" => generate_bindings(),
        "all" => build_sdk().and_then(|_| generate_bindings()),
        "w800-sdk" => w800::build_sdk(),
        "w800-bindgen" => w800::generate_bindings(),
        "w800-all" => w800::build_sdk().and_then(|_| w800::generate_bindings()),
        _ => {
            eprintln!("用法: cargo xtask <sdk|bindgen|all|w800-sdk|w800-bindgen|w800-all>");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn work_dir() -> PathBuf {
    workspace_root().join("target").join("w600-sdk")
}

fn tool(name: &str) -> PathBuf {
    let exe = format!("arm-none-eabi-{name}{}", env::consts::EXE_SUFFIX);
    match env::var_os("W600_TOOLCHAIN") {
        Some(dir) => PathBuf::from(dir).join(exe),
        None => PathBuf::from(exe),
    }
}

fn run(cmd: &mut Command) -> Result<String> {
    let out = cmd
        .output()
        .map_err(|e| format!("无法执行 {:?}: {e}", cmd.get_program()))?;
    if !out.status.success() {
        return Err(format!(
            "{:?} 执行失败:\n{}{}",
            cmd.get_program(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 返回 SDK 根目录，必要时下载、校验并解压。
fn ensure_sdk() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("W600_SDK_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let work = work_dir();
    let sdk = work.join(SDK_DIR_NAME);
    if sdk.join("Include").is_dir() {
        return Ok(sdk);
    }
    fs::create_dir_all(&work)?;
    let zip_path = work.join("sdk.zip");
    download_verified(SDK_URL, SDK_SHA256, &zip_path)?;
    println!("解压 SDK 到 {}", sdk.display());
    zip::ZipArchive::new(fs::File::open(&zip_path)?)?.extract(&sdk)?;
    Ok(sdk)
}

/// 文件不存在时下载，然后校验 SHA-256，校验失败会删除文件。
fn download_verified(url: &str, sha256: &str, path: &Path) -> Result<()> {
    if !path.exists() {
        println!("下载 SDK: {url}");
        run(Command::new("curl").args(["-L", "--fail", "-o"]).arg(path).arg(url))?;
    }
    let digest = Sha256::digest(fs::read(path)?);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    if hex != sha256 {
        fs::remove_file(path)?;
        return Err(format!("SDK 压缩包校验失败: {hex}").into());
    }
    Ok(())
}

/// 从模块 Makefile 中取出 C_SRC/ASM_SRC，只保留 GNU 工具链分支下的条目。
fn parse_sources(makefile: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(makefile)?.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut active: Vec<bool> = Vec::new();
    let mut sources = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(cond) = line.strip_prefix("ifeq").or_else(|| line.strip_prefix("ifneq")) {
            let truth = cond.contains("TOOL_GNU");
            active.push(if line.starts_with("ifneq") { !truth } else { truth });
        } else if line.starts_with("ifdef") || line.starts_with("ifndef") {
            active.push(false);
        } else if line == "else" {
            if let Some(top) = active.last_mut() {
                *top = !*top;
            }
        } else if line == "endif" {
            active.pop();
        } else if active.iter().all(|a| *a) {
            for var in ["C_SRC", "ASM_SRC"] {
                if let Some(rest) = line.strip_prefix(var) {
                    let rest = rest.trim_start();
                    if let Some(files) = ["+=", ":=", "="].iter().find_map(|op| rest.strip_prefix(op)) {
                        sources.extend(files.split_whitespace().map(str::to_string));
                    }
                }
            }
        }
    }
    Ok(sources)
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

struct Job {
    src: PathBuf,
    obj: PathBuf,
}

fn apply_config(sdk: &Path) -> Result<()> {
    let path = sdk.join("Include/wm_config.h");
    let text = fs::read_to_string(&path)?;
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let name = match (words.next(), words.next()) {
            (Some("#define"), Some(name)) => name,
            _ => "",
        };
        match CONFIG_OVERRIDES.iter().find(|(n, _)| *n == name) {
            Some((n, v)) => out.push_str(&format!("#define {n} {v}")),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    if out != text {
        fs::write(&path, out)?;
    }
    Ok(())
}

/// 启动文件的中断向量表段没有写分配标志。GNU ld 仍会沿着 KEEP 段的引用保留中断处理函数，
/// rust-lld 则不会追踪不可分配段里的引用，--gc-sections 会把只被向量表引用的
/// SVC_Handler、PendSV_Handler、各外设中断等全部去掉，向量表中对应的项变成 0。
fn patch_startup(sdk: &Path) -> Result<()> {
    let path = sdk.join("Platform/Boot/gcc/startup_ARMCM3.S");
    let text = fs::read_to_string(&path)?;
    let out = text.replace(".section .isr_vector\n", ".section .isr_vector,\"a\",%progbits\n");
    if out != text {
        fs::write(&path, out)?;
    }
    Ok(())
}

fn build_sdk() -> Result<()> {
    let sdk = ensure_sdk()?;
    apply_config(&sdk)?;
    patch_startup(&sdk)?;
    let root = workspace_root();
    let obj_dir = work_dir().join("obj");
    let gcc = tool("gcc");
    let incs = include_args(&sdk);
    let file_prefix_map = format!("-ffile-prefix-map={}={SDK_DIR_NAME}", sdk.display());

    let mut jobs = Vec::new();
    for module in MODULES {
        let dir = sdk.join(module);
        for src in parse_sources(&dir.join("Makefile"))? {
            let obj_name = format!("{}__{}", module.replace('/', "_"), src.replace('/', "_"));
            jobs.push(Job {
                src: dir.join(&src),
                obj: obj_dir.join(obj_name).with_extension("o"),
            });
        }
    }
    let stamp = format!("{gcc:?}{CPU_FLAGS:?}{C_FLAGS:?}{CONFIG_OVERRIDES:?}{incs:?}{file_prefix_map:?}");
    let stamp_path = obj_dir.join("flags.stamp");
    if fs::read_to_string(&stamp_path).ok().as_deref() != Some(stamp.as_str()) {
        let _ = fs::remove_dir_all(&obj_dir);
    }
    fs::create_dir_all(&obj_dir)?;
    fs::write(&stamp_path, &stamp)?;

    println!("编译 {} 个源文件 ...", jobs.len());
    let next = Mutex::new(jobs.iter());
    let errors = Mutex::new(Vec::new());
    let workers = thread::available_parallelism().map_or(4, |n| n.get());
    thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let Some(job) = next.lock().unwrap().next() else { break };
                    if is_up_to_date(&job.src, &job.obj) {
                        continue;
                    }
                    let mut cmd = Command::new(&gcc);
                    // 固定 __DATE__/__TIME__（SDK 发布时间），并把 __FILE__ 中的 SDK 绝对路径换成目录名，
                    // 保证在任何目录下重复构建都得到相同的库。
                    cmd.env("SOURCE_DATE_EPOCH", "1584693446");
                    cmd.arg(&file_prefix_map);
                    cmd.args(CPU_FLAGS).args(C_FLAGS).args(&incs);
                    if job.src.extension().is_some_and(|e| e == "S" || e == "s") {
                        cmd.arg("-x").arg("assembler-with-cpp");
                    }
                    cmd.arg("-c").arg(&job.src).arg("-o").arg(&job.obj);
                    if let Err(e) = run(&mut cmd) {
                        errors.lock().unwrap().push(format!("{}\n{e}", job.src.display()));
                    }
                }
            });
        }
    });
    let errors = errors.into_inner().unwrap();
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("{e}");
        }
        return Err(format!("{} 个文件编译失败", errors.len()).into());
    }

    let lib_dir = root.join("w600-sys").join("lib");
    fs::create_dir_all(&lib_dir)?;
    let lib = lib_dir.join("libw600sdk.a");
    if lib.exists() {
        fs::remove_file(&lib)?;
    }
    // Windows 命令行长度有限，用响应文件把目标文件列表传给 ar。
    let rsp = work_dir().join("objects.rsp");
    let list: Vec<String> = jobs
        .iter()
        .map(|j| format!("\"{}\"", j.obj.display().to_string().replace('\\', "/")))
        .collect();
    fs::write(&rsp, list.join("\n"))?;
    // D：不记录时间戳和 uid，保证重复构建得到相同的文件。
    run(Command::new(tool("ar")).arg("rcsD").arg(&lib).arg(format!("@{}", rsp.display())))?;

    fs::copy(sdk.join("Lib/GNU/wlan.a"), lib_dir.join("libwlan.a"))?;
    fs::copy(
        sdk.join("Src/App/oneshotconfig/lib_gcc/libairkiss_log.a"),
        lib_dir.join("libairkiss_log.a"),
    )?;
    // 随 crate 提供 newlib 和 libgcc（Cortex-M3 软浮点版本），使用者用 rust-lld 链接即可，无需安装 ARM 工具链。
    for name in ["libc.a", "libm.a", "libgcc.a"] {
        let src = run(Command::new(&gcc).args(CPU_FLAGS).arg(format!("-print-file-name={name}")))?;
        run(Command::new(tool("strip"))
            .args(["--strip-debug", "-D"])
            .arg(&src)
            .arg("-o")
            .arg(lib_dir.join(name)))?;
    }

    let bin_dir = root.join("w600-sys").join("bin");
    fs::create_dir_all(&bin_dir)?;
    fs::copy(sdk.join("Bin/secboot.img"), bin_dir.join("secboot.img"))?;
    let tool_dir = root.join("tools").join("wm_tool");
    fs::create_dir_all(&tool_dir)?;
    fs::copy(sdk.join("Tools/wm_tool.exe"), tool_dir.join("wm_tool.exe"))?;
    fs::copy(sdk.join("Tools/src/wm_tool.c"), tool_dir.join("wm_tool.c"))?;

    println!("完成: {}", lib.display());
    Ok(())
}

fn is_up_to_date(src: &Path, obj: &Path) -> bool {
    let mtime = |p: &Path| fs::metadata(p).and_then(|m| m.modified());
    matches!((mtime(src), mtime(obj)), (Ok(s), Ok(o)) if o >= s)
}

fn generate_bindings() -> Result<()> {
    let sdk = ensure_sdk()?;
    apply_config(&sdk)?;
    let root = workspace_root();
    let gcc = tool("gcc");
    let sysroot = run(Command::new(&gcc).arg("-print-sysroot"))?;
    let gcc_inc = run(Command::new(&gcc).arg("-print-file-name=include"))?;
    let gcc_inc_fixed = run(Command::new(&gcc).arg("-print-file-name=include-fixed"))?;

    let mut clang_args: Vec<String> = [
        "--target=thumbv7m-none-eabi",
        "-mcpu=cortex-m3",
        "-mfloat-abi=soft",
        // SDK（包括闭源的 wlan.a）按 AAPCS 短枚举编译，见 readelf 的 Tag_ABI_enum_size: small。
        "-fshort-enums",
        "-std=gnu99",
        "-nostdinc",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for dir in [&gcc_inc, &gcc_inc_fixed, &format!("{sysroot}/include")] {
        clang_args.push(format!("-isystem{dir}"));
    }
    clang_args.extend(include_args(&sdk));

    let bindings = bindgen::Builder::default()
        .header(root.join("w600-sys/wrapper.h").display().to_string())
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
        .allowlist_function(FUNCTIONS)
        .allowlist_type(TYPES)
        .allowlist_var(VARS)
        .generate()
        .map_err(|e| format!("bindgen 失败: {e}"))?;

    let out = root.join("w600-sys/src/bindings.rs");
    let header = "// 由 `cargo xtask bindgen` 从 WM_SDK_W60X_G3.04.00 头文件生成，请勿手动修改。\n\n";
    fs::write(&out, format!("{header}{bindings}"))?;
    println!("完成: {}", out.display());
    Ok(())
}

const FUNCTIONS: &str = concat!(
    "(tls|wm|adc)_\\w+|mem_(alloc|free|realloc|calloc)_debug|printf|",
    "socket|bind|listen|accept|connect|shutdown|closesocket|send|sendto|recv|recvfrom|",
    "setsockopt|getsockopt|getsockname|getpeername|select|ioctlsocket|gethostbyname|ipaddr_addr"
);

const TYPES: &str = "(tls|wm|TLS|WM)_\\w+|sockaddr\\w*|in_addr|hostent|timeval|fd_set|linger|ip4?_addr\\w*";

const VARS: &str = concat!(
    "(TLS|WM|NETIF|CPU_CLK|AF|PF|SOCK|IPPROTO|SOL|SO|TCP|MSG|SHUT|INADDR)_\\w*|",
    "E[A-Z0-9]+|errno|F_GETFL|F_SETFL|O_NONBLOCK|USER_ADDR_START|USER_AREA_LEN|ADC_\\w+"
);