//! W600 固件打包与串口烧录工具，可直接用作 cargo runner（`runner = "w600-flash"`）。
//!
//! 流程：ELF --> .bin --wm_tool--> .img / _gz.img / .fls --wm_tool--> 串口下载。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const SECBOOT: &[u8] = include_bytes!("../../w600-sys/bin/secboot.img");
#[cfg(windows)]
const WM_TOOL: &[u8] = include_bytes!("../../tools/wm_tool/wm_tool.exe");
#[cfg(not(windows))]
const WM_TOOL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wm_tool"));

const USAGE: &str = "\
用法: w600-flash [选项] <ELF>

选项:
  -p, --port <串口>     例如 COM6 或 /dev/ttyUSB0；默认读取环境变量 W600_PORT。
                        未指定串口时只生成镜像，不烧录。
  -b, --baud <波特率>   下载波特率，默认 2000000
      --reset <方式>    下载前复位芯片的方式：rts（默认）、at、none（手动按复位键）
  -m, --monitor         烧录完成后显示串口输出（115200）
  -h, --help            显示帮助

环境变量:
  W600_PORT       默认串口
  W600_WM_TOOL    使用指定的 wm_tool 可执行文件，而不是内置的版本";

struct Args {
    elf: PathBuf,
    port: Option<String>,
    baud: String,
    reset: String,
    monitor: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut elf = None;
    let mut port = env::var("W600_PORT").ok().filter(|p| !p.is_empty());
    let mut baud = "2000000".to_string();
    let mut reset = "rts".to_string();
    let mut monitor = false;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} 缺少参数"));
        match arg.as_str() {
            "-p" | "--port" => port = Some(value("--port")?),
            "-b" | "--baud" => baud = value("--baud")?,
            "--reset" => reset = value("--reset")?,
            "-m" | "--monitor" => monitor = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ if elf.is_none() && !arg.starts_with('-') => elf = Some(PathBuf::from(arg)),
            _ => return Err(format!("无法识别的参数: {arg}")),
        }
    }
    let elf = elf.ok_or("缺少 ELF 文件路径")?;
    Ok(Args { elf, port, baud, reset, monitor })
}

/// 应用镜像在 Flash 中允许的地址范围（与 w600.x 一致）。
const APP_FLASH: std::ops::Range<u32> = 0x0801_0100..0x0809_0000;

/// 等价于 `objcopy -O binary`：取所有带文件内容的 PT_LOAD 段，按物理地址（LMA）拼接，空隙填 0。
fn elf_to_bin(elf: &[u8]) -> Result<Vec<u8>, String> {
    let bytes = |off: usize, len: usize| elf.get(off..off + len).ok_or("ELF 文件不完整");
    let u16_at = |off: usize| bytes(off, 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |off: usize| bytes(off, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));

    if elf.len() < 52 || &elf[..4] != b"\x7fELF" || elf[4] != 1 || elf[5] != 1 {
        return Err("不是 32 位小端 ELF 文件".into());
    }
    let phoff = u32_at(28)? as usize;
    let phentsize = u16_at(42)? as usize;
    let phnum = u16_at(44)? as usize;

    let mut segments = Vec::new();
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        const PT_LOAD: u32 = 1;
        let filesz = u32_at(ph + 16)? as usize;
        if u32_at(ph)? != PT_LOAD || filesz == 0 {
            continue;
        }
        let paddr = u32_at(ph + 12)?;
        if !APP_FLASH.contains(&paddr) || paddr as u64 + filesz as u64 > APP_FLASH.end as u64 {
            return Err(format!("段地址 {paddr:#x} 超出 Flash 应用区，请检查链接脚本"));
        }
        segments.push((paddr, bytes(u32_at(ph + 4)? as usize, filesz)?));
    }

    let start = segments.iter().map(|s| s.0).min().ok_or("ELF 中没有可加载的段")?;
    let end = segments.iter().map(|s| s.0 as usize + s.1.len()).max().unwrap_or(0);
    let mut bin = vec![0u8; end - start as usize];
    for (addr, data) in segments {
        let off = (addr - start) as usize;
        bin[off..off + data.len()].copy_from_slice(data);
    }
    Ok(bin)
}

fn wm_tool() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("W600_WM_TOOL") {
        return Ok(PathBuf::from(path));
    }
    let dir = env::temp_dir().join(format!("w600-flash-{}", WM_TOOL.len()));
    let exe = dir.join(format!("wm_tool{}", env::consts::EXE_SUFFIX));
    if fs::read(&exe).ok().as_deref() != Some(WM_TOOL) {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::write(&exe, WM_TOOL).map_err(|e| format!("无法写出 wm_tool: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        }
    }
    Ok(exe)
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let status = cmd.status().map_err(|e| format!("无法执行 {:?}: {e}", cmd.get_program()))?;
    if status.success() { Ok(()) } else { Err(format!("{:?} 执行失败（{status}）", cmd.get_program())) }
}

/// wm_tool 只按 '/' 拆分路径。
fn slash(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn flash(args: Args) -> Result<(), String> {
    let wm_tool = wm_tool()?;
    let dir = args.elf.parent().unwrap_or(Path::new("."));
    let name = args.elf.file_stem().ok_or("ELF 路径无效")?.to_string_lossy().into_owned();
    let bin = dir.join(format!("{name}.bin"));
    let out = dir.join(&name);
    let secboot = dir.join("secboot.img");
    if fs::read(&secboot).ok().as_deref() != Some(SECBOOT) {
        fs::write(&secboot, SECBOOT).map_err(|e| e.to_string())?;
    }

    let elf = fs::read(&args.elf).map_err(|e| format!("无法读取 {}: {e}", args.elf.display()))?;
    fs::write(&bin, elf_to_bin(&elf)?).map_err(|e| e.to_string())?;
    run(Command::new(&wm_tool)
        .args(["-b", &slash(&bin), "-sb", &slash(&secboot)])
        .args(["-fc", "compress", "-it", "1M", "-ua", "90000", "-ra", "10100"])
        .args(["-o", &slash(&out)]))?;
    let image = format!("{}_gz.img", slash(&out));
    println!("镜像: {image}（首次烧录或救砖用 {}.fls）", slash(&out));

    let Some(port) = args.port else {
        println!("未指定串口（--port 或环境变量 W600_PORT），跳过烧录");
        return Ok(());
    };
    // Linux / macOS 上 wm_tool 会自己加上 /dev/ 前缀。
    let port = port.strip_prefix("/dev/").unwrap_or(&port).to_string();
    let mut cmd = Command::new(&wm_tool);
    cmd.args(["-c", &port, "-ds", &args.baud, "-rs", &args.reset, "-dl", &image]);
    if args.monitor {
        cmd.args(["-ws", "115200", "-sl", "str"]);
    }
    run(&mut cmd)
}

fn main() -> ExitCode {
    let result = parse_args().and_then(flash);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("w600-flash: {e}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}
