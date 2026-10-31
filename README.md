# lib_w600：用 Rust 开发联盟微 W600

在官方 SDK（WM_SDK_W60X_G3.04.00）之上提供的 Rust 开发库。FreeRTOS、Wi-Fi 协议栈、lwIP 和外设驱动继续使用官方实现，
Rust 一侧提供符合习惯的安全接口，并实现 `embedded-hal` 1.0 / `embedded-io` 0.7，可以直接使用社区里的驱动 crate。

- **只需要安装 stable Rust**，不需要 nightly
- **不需要安装 SDK、ARM 工具链、libclang 或 make**：SDK 和 newlib 已预编译为静态库，FFI 绑定已由 bindgen 生成，
  链接使用 Rust 自带的 rust-lld，都随仓库提供
- `cargo run` 一条命令完成编译、打包和串口烧录

| 目录 | 说明 |
| --- | --- |
| `w600-sys` | bindgen 生成的原始绑定、预编译静态库（SDK + newlib）、链接脚本 `w600.x` |
| `w600` | 运行时、线程与同步、GPIO/PWM/UART/I2C/SPI/ADC/定时器/Flash、Wi-Fi、TCP/UDP |
| `w600-flash` | 打包与烧录工具，作为 cargo runner 使用 |
| `examples` | 示例，同时也是新项目的模板 |
| `xtask` | 维护者工具：下载并编译 SDK、重新生成绑定 |

## 快速开始

1. 安装 Rust（stable）和编译目标：

   ```sh
   rustup target add thumbv7m-none-eabi
   ```

2. 安装烧录工具：

   ```sh
   cargo install --path w600-flash
   ```

   它内置了官方的 `wm_tool`：Windows 上直接使用 SDK 附带的 `wm_tool.exe`；Linux / macOS 上安装时会用本机的
   C 编译器（gcc / clang，一般系统自带）自动编译 `tools/wm_tool/wm_tool.c`。

3. 连接开发板，编译并烧录示例：

   ```powershell
   cd examples
   $env:W600_PORT = "COM6"          # Linux: export W600_PORT=/dev/ttyUSB0
   cargo run --release --bin blinky -- --monitor
   ```

   不设置 `W600_PORT` 时只生成镜像（`target/thumbv7m-none-eabi/release/blinky_gz.img`）。
   如果开发板的 RTS 没有接到复位脚，加上 `--reset none`，在工具提示同步时手动按复位键。

新项目直接复制 `examples` 目录，把 `Cargo.toml` 里的 `path` 依赖改成实际位置即可。
关键是 `.cargo/config.toml` 中的这几行：

```toml
[build]
target = "thumbv7m-none-eabi"

[target.thumbv7m-none-eabi]
runner = "w600-flash"
rustflags = ["-C", "link-arg=-Tw600.x"]
```

如果更想用 GNU ld，也可以设置 `linker = "arm-none-eabi-gcc"`，并在 rustflags 中额外加入
`-nostdlib`、`-mcpu=cortex-m3`、`-mthumb`（都以 `link-arg=` 形式传入），链接脚本两者通用。

## 示例

```rust
#![no_std]
#![no_main]

use core::time::Duration;
use embedded_io::{Read, Write};
use w600::{gpio::{Level, Output}, net::TcpStream, println, thread, wifi};

w600::entry!(main);

fn main() {
    let p = w600::Peripherals::take().unwrap();
    let mut led = Output::new(p.pins.pb16, Level::High);
    thread::spawn(move || loop {
        led.toggle();
        thread::sleep(Duration::from_millis(500));
    });

    let info = wifi::connect("ssid", "password", Duration::from_secs(20)).unwrap();
    println!("ip = {}", info.ip);

    let mut s = TcpStream::connect_host("example.com", 80).unwrap();
    s.write_all(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n").unwrap();
    let mut buf = [0u8; 256];
    let n = s.read(&mut buf).unwrap();
    println!("{}", core::str::from_utf8(&buf[..n]).unwrap_or("?"));
}
```

`examples/src/bin` 下还有：`threads`（线程、锁、通道）、`tcp_server`（多线程回显服务器）、
`peripherals`（PWM 呼吸灯、按键中断、UART 回显、ADC 温度、Flash 读写）。

## 需要了解的限制

这些限制来自 SDK 或芯片本身，库已经做了处理，但写代码时需要知道：

- **入口在线程中运行**：SDK 要求 `UserMain` 尽快返回，`entry!` 会在一个新线程（默认 8KB 栈）里运行你的 `main`，所以里面可以写死循环。
- **线程不会被删除**：SDK 的 FreeRTOS 删除任务时会用错误的堆释放任务栈。执行完的线程会进入空闲池，被后续的 `spawn` 复用。
- **优先级数值越小越高**：SDK 内部任务使用 0～28，应用线程允许 30～62，默认 40；同优先级的线程按时间片轮转。
- **任务栈必须位于 0x20028000 以下**，否则 `spawn` 返回错误。栈从堆上分配，尽量在程序启动时创建长期运行的线程。
- **中断回调**（GPIO 中断、硬件定时器）不能阻塞、不能加锁，可以用 `Channel::send` / `Semaphore::release` 通知线程。
  `sync` 里的对象第一次使用必须在线程中，或者先调用 `init()`。
- **时间精度 2ms**（系统节拍 500Hz），`sleep` / 超时向上取整。
- **信号量计数上限为 5**，**同时最多 8 个 socket**（SDK 的 FreeRTOS / lwIP 配置）。
- **UART0 是调试串口**，供 `println!` 使用；应用可以使用 UART1 / UART2。
- **SPI 由硬件控制片选**，`SpiDevice` 只支持 `[Write]`、`[Read]`、`[Write, Read]`、`[Write, Write]` 这几种组合，不支持全双工。
- **Flash 用户区共 44KB**（`flash::UserFlash`），写入时 SDK 会自动完成擦除。
- 堆约 160KB，与 C 代码共享（`system::free_heap()` 可查看剩余量）；应用代码区 512KB，Wi-Fi + TCP 的示例约占 330KB。
- 预编译库**关闭了 SDK 的 AT 指令子系统**（`TLS_CONFIG_HOSTIF`），可节省约 150KB Flash。

## 维护：重新生成静态库和绑定

```sh
cargo xtask all        # 等价于 cargo xtask sdk && cargo xtask bindgen
```

`xtask` 会：

1. 从联盟微官网下载 SDK 并校验 SHA-256（也可以用 `W600_SDK_DIR` 指向已解压或修改过的 SDK）；
2. 修改 `Include/wm_config.h`（目前只关闭 `TLS_CONFIG_HOSTIF`）；
3. 解析各模块的 Makefile 得到源文件列表，直接调用 `arm-none-eabi-gcc` 并行编译（不需要 make 或 bash），
   打包为 `w600-sys/lib/libw600sdk.a`，并复制闭源的 `libwlan.a`、`libairkiss_log.a`、`secboot.img`、`wm_tool`，
   以及工具链中 Cortex-M3 软浮点版的 `libc.a`、`libm.a`、`libgcc.a`（去掉调试信息）；
4. 用 bindgen 从 `w600-sys/wrapper.h` 生成 `w600-sys/src/bindings.rs`。

维护者需要 [Arm GNU Toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads)（在 `PATH` 中，或用 `W600_TOOLCHAIN` 指定 `bin` 目录）
和 libclang（设置 `LIBCLANG_PATH`），使用者都不需要。同一环境下重复构建得到的库文件完全相同。几个容易出错的细节已经在 xtask 中处理：

- SDK 和闭源的 `wlan.a` 都按 AAPCS **短枚举**编译（`Tag_ABI_enum_size: small`），bindgen 必须加 `-fshort-enums`，否则结构体布局不一致；
- socket 的错误码来自 lwIP 自己的 errno 定义，与 newlib 的部分取值不同，因此不包含 newlib 的 `<errno.h>`；
- 新版 GCC 下编译老 SDK 需要 `-fcommon -fpermissive`，并定义 `__USE_W32_SOCKETS`，避免 newlib 重复定义 `fd_set`；
- 启动文件的 `.stack` / `.heap` 段没有分配标志，原版脚本依赖 GNU ld 的 `(COPY)`，rust-lld 会把它们放到地址 0。
  因此 `w600.x` 自己预留主栈并定义栈和堆的符号，启动文件按 `__HEAP_SIZE=0` 编译。

## 当前状态

所有示例都能在 stable Rust 下用 rust-lld 编译、链接（Windows 上验证；烧录工具在 Linux 上也验证过），
生成的镜像与官方工具链产物格式一致。**目前还没有在真实硬件上测试过**，欢迎反馈问题。

## 许可

Rust 代码以 MIT 或 Apache-2.0 授权。`w600-sys/bin`、`tools/wm_tool` 以及 `w600-sys/lib` 中的 SDK 库
（`libw600sdk.a`、`libwlan.a`、`libairkiss_log.a`）来自联盟微官方 SDK，版权归北京联盛德微电子有限责任公司所有；
`libc.a`、`libm.a` 来自 newlib（BSD 类许可），`libgcc.a` 来自 GCC（GPL 附运行库例外，允许链接进任意程序）。
