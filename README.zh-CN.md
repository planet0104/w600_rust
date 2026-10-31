# lib_w600：用 Rust 开发联盛德 W600 / W800

中文 | [English](README.md)

在官方 SDK（W600：WM_SDK_W60X_G3.04.00；W800：W80X_SDK_v1.00.10）之上提供的 Rust 开发库。
FreeRTOS、Wi-Fi 协议栈、lwIP 和外设驱动继续使用官方实现，Rust 一侧提供符合习惯的安全接口，
并实现 `embedded-hal` 1.0 / `embedded-io` 0.7，可以直接使用社区里的驱动 crate。

- 两款芯片共用同一个 `w600` crate 和同一套接口，芯片由编译目标自动决定，个别差异在文档中注明
- **W600 只需要安装 stable Rust**，不需要 SDK、ARM 工具链、libclang 或 make：SDK 和 newlib 已预编译为静态库，
  FFI 绑定已由 bindgen 生成，链接使用 Rust 自带的 rust-lld，都随仓库提供
- **W800 的 CPU 是平头哥 XT804（C-SKY 架构）**，Rust 没有官方的 C-SKY 裸机目标，rust-lld 也不支持 C-SKY，
  因此需要 nightly Rust（由 `examples-w800/rust-toolchain.toml` 自动选择）和平头哥的 `csky-elfabiv2-gcc` 工具链（只用来链接），
  详见 [W800](#w800)
- `cargo run` 一条命令完成编译、打包和串口烧录

| 目录 | 说明 |
| --- | --- |
| `w600-sys` | W600：bindgen 生成的原始绑定、预编译静态库（SDK + newlib）、链接脚本 `w600.x` |
| `w800-sys` | W800：bindgen 生成的原始绑定、SDK 自带的预编译静态库、链接脚本 `w800.x` |
| `w600` | 运行时、线程与同步、GPIO/PWM/UART/I2C/SPI/ADC/定时器/Flash、Wi-Fi、TCP/UDP（两款芯片通用） |
| `w600-flash` | 打包与烧录工具，作为 cargo runner 使用，按 ELF 架构自动识别芯片 |
| `examples` | W600 示例，同时也是新项目的模板 |
| `examples-w800` | W800 示例与模板（含 C-SKY 目标描述 `csky-none-elfabiv2hf.json`） |
| `xtask` | 维护者工具：下载 SDK、生成静态库和绑定 |
| `third_party/embedded-tls` | TLS 1.3 库 embedded-tls 0.19.0，附一处兼容性修复和一处栈用量修复 |

## 快速开始（W600）

1. 安装 Rust（stable）和编译目标：

   ```sh
   rustup target add thumbv7m-none-eabi
   ```

2. 安装烧录工具：

   ```sh
   cargo install --path w600-flash
   ```

   它内置了 W600 和 W800 两个版本的官方 `wm_tool`：Windows 上直接使用 SDK 附带的 `wm_tool.exe`；Linux / macOS 上
   安装时会用本机的 C 编译器（gcc / clang，一般系统自带）自动编译 `tools/` 下的 `wm_tool.c`（W800 版连同随附的 zlib）。

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

`examples/src/bin`（以及 `examples-w800/src/bin`）下还有：`threads`（线程、锁、通道）、`tcp_server`（多线程回显服务器）、
`tcp_client`、`https_client`（TLS 1.3 访问 HTTPS 网站）、`peripherals`（PWM 呼吸灯、按键中断、UART 回显、ADC 温度、Flash 读写）、
`servo`（输出舵机信号，W600 用 PB15、W800 用 PB3，转到几个固定角度后来回扫动）。
每个 PWM 通道只能使用固定的几个引脚（见 `Pwm::new` 的文档），选错时 `Pwm::new` 返回 `InvalidPin`。

## HTTPS / TLS

`w600::tls::TlsStream` 是基于 [embedded-tls](https://crates.io/crates/embedded-tls) 的 TLS 1.3 客户端，两款芯片通用，
建立在 `net::TcpStream` 之上，同样实现 `embedded-io` 的 `Read` / `Write`：

```rust
let mut buffers = w600::tls::Buffers::new();                       // 约 20KB 记录缓冲区，可复用
let mut s = w600::tls::TlsStream::connect("www.nmc.cn", 443, None, &mut buffers)?;
s.write_all(b"GET /rest/weather?stationid=2011 HTTP/1.1\r\nHost: www.nmc.cn\r\nConnection: close\r\n\r\n")?;
s.flush()?;                                                          // 写入的数据在 flush 后才发出
```

- 只支持 TLS 1.3（TLS_AES_128_GCM_SHA256，P-256 密钥交换），自动发送 SNI；只支持 TLS 1.2 的服务器
  （例如 www.baidu.com）无法连接，握手会报 `HandshakeAborted(Fatal, ProtocolVersion)`。
- embedded-tls 0.19.0 遇到服务器列出它不认识的密钥交换组（如 brainpool、国密 SM2）时会握手失败（`DecodeError`），
  国内很多网站如此。仓库在 `third_party/embedded-tls` 中附带了修复此问题的副本（与上游 main 分支的修复相同）。
  该副本还给证书校验函数加了 `#[inline(never)]`：否则校验代码被内联进握手函数，即使不校验证书也要多占约 20KB 栈。
- 第三个参数为 `None` 时**不校验服务器证书**：通信是加密的，但无法防止中间人攻击。传入 DER 格式的根证书时，
  校验证书链和域名（没有实时时钟，不检查有效期）。可校验的签名算法：W800 为 ECDSA P-256 与 RSA，
  W600 受 512KB 应用区限制只有 ECDSA P-256。
- 每个连接约占 30KB 堆内存。握手需要较大的线程栈：不校验证书时 24KB 足够，校验证书时需要约 48KB（示例按是否设置 `CA` 自动选择）；栈不够时芯片会直接卡死而不报错。代码体积约 100KB（W600）/ 200KB（W800）。
- 随机数：W800 使用硬件真随机数发生器；W600 没有，种子取自芯片温度 ADC 噪声与 CPU 计时抖动，强度较弱。

## 需要了解的限制

这些限制来自 SDK 或芯片本身，库已经做了处理，但写代码时需要知道。以下以 W600 为准，W800 的差异见 [W800](#w800)：

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

## 维护：重新生成静态库和绑定（W600）

```sh
cargo xtask all        # 等价于 cargo xtask sdk && cargo xtask bindgen
```

`xtask` 会：

1. 从联盛德官网下载 SDK 并校验 SHA-256（也可以用 `W600_SDK_DIR` 指向已解压或修改过的 SDK）；
2. 修改 `Include/wm_config.h`（目前只关闭 `TLS_CONFIG_HOSTIF`）；
3. 解析各模块的 Makefile 得到源文件列表，直接调用 `arm-none-eabi-gcc` 并行编译（不需要 make 或 bash），
   打包为 `w600-sys/lib/libw600sdk.a`，并复制闭源的 `libwlan.a`、`libairkiss_log.a`、`secboot.img`、`wm_tool`，
   以及工具链中 Cortex-M3 软浮点版的 `libc.a`、`libm.a`、`libgcc.a`（去掉调试信息）；
4. 用 bindgen 从 `w600-sys/wrapper.h` 生成 `w600-sys/src/bindings.rs`。

维护者需要 [Arm GNU Toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads)（在 `PATH` 中，或用 `W600_TOOLCHAIN` 指定 `bin` 目录）
和 libclang（设置 `LIBCLANG_PATH`），使用者都不需要。现有的库用 Arm GNU Toolchain 15.2.Rel1 生成，
使用同一版本重复构建得到的库文件完全相同。几个容易出错的细节已经在 xtask 中处理：

- SDK 和闭源的 `wlan.a` 都按 AAPCS **短枚举**编译（`Tag_ABI_enum_size: small`），bindgen 必须加 `-fshort-enums`，否则结构体布局不一致；
- socket 的错误码来自 lwIP 自己的 errno 定义，与 newlib 的部分取值不同，因此不包含 newlib 的 `<errno.h>`；
- 新版 GCC 下编译老 SDK 需要 `-fcommon -fpermissive`，并定义 `__USE_W32_SOCKETS`，避免 newlib 重复定义 `fd_set`；
- 启动文件的 `.stack` / `.heap` 段没有分配标志，原版脚本依赖 GNU ld 的 `(COPY)`，rust-lld 会把它们放到地址 0。
  因此 `w600.x` 自己预留主栈并定义栈和堆的符号，启动文件按 `__HEAP_SIZE=0` 编译；
- 启动文件的中断向量表段 `.isr_vector` 同样没有分配标志。GNU ld 仍会沿着 `KEEP` 段的引用保留中断处理函数，
  rust-lld 不会，`--gc-sections` 会去掉只被向量表引用的 `SVC_Handler`、`PendSV_Handler`、各外设中断等，
  向量表中对应的项变成 0，FreeRTOS 一启动就跑飞。因此 xtask 编译前会给该段加上 `"a"` 标志；
- 用 `-ffile-prefix-map` 把 `__FILE__` 中的 SDK 绝对路径换成目录名，在不同机器、不同目录下构建的库也完全相同；
- lwIP 的 `TCP_MSS` 从 1024 改为 1460，`TCP_WND` 从 6 × MSS 改为 8 × MSS，与 W800 SDK 相同。部分服务器的前端
  （如 www.nmc.cn）在通告窗口低于约 8KB 时停止发送，原配置下 TLS 握手会一直卡住。lwIP 的缓冲区从堆上按需分配，
  不增加静态内存。

## W800

### 快速开始

1. 安装平头哥的 C-SKY 工具链（800 系列，minilibc 版），把它的 `bin` 目录加入 `PATH`：
   - 官方下载：[平头哥芯片开放社区](https://occ.t-head.cn/community/download?id=616214177698021376)（需要注册），
     Windows 选 `csky-elfabiv2-tools-mingw-minilibc-*.tar.gz`，Linux 选 `csky-elfabiv2-tools-x86_64-minilibc-*.tar.gz`
   - 本仓库使用 `20210423` 版本验证；无法注册时可以用社区的[同版本备份](https://github.com/crosstyan/Xuantie-800-gcc-elf-newlib-prebuilt/releases/tag/v0.00)

   Windows 上可以直接用下面的命令安装到用户目录（不要放在 `C:\` 根目录，普通用户没有写权限）：

   ```powershell
   $dir = "$env:USERPROFILE\csky-toolchain"
   New-Item -ItemType Directory -Force $dir | Out-Null
   curl.exe -L -o "$env:TEMP\csky.tar.gz" "https://github.com/crosstyan/Xuantie-800-gcc-elf-newlib-prebuilt/releases/download/v0.00/csky-elfabiv2-tools-mingw-minilibc-20210423.tar.gz"
   tar -xzf "$env:TEMP\csky.tar.gz" -C $dir
   # 永久加入用户 PATH（新开的终端生效），并在当前终端立即生效
   [Environment]::SetEnvironmentVariable("Path", "$dir\bin;" + [Environment]::GetEnvironmentVariable("Path", "User"), "User")
   $env:PATH = "$dir\bin;$env:PATH"
   csky-elfabiv2-gcc --version      # 应输出 T-HEAD C-SKY Tools V3.10.29 ... B20210423
   ```

   Linux 上下载 `csky-elfabiv2-tools-x86_64-minilibc-20210423.tar.gz`，解压后把 `bin` 加入 `PATH` 即可。
   不想修改 `PATH` 时，也可以用环境变量 `CARGO_TARGET_CSKY_NONE_ELFABIV2HF_LINKER` 指定 `csky-elfabiv2-gcc` 的完整路径。
   编译时出现 ``linker `csky-elfabiv2-gcc` not found`` 就是这一步没有完成。

2. 安装烧录工具（与 W600 相同，已安装过的需要重新安装以加入 W800 支持）：

   ```sh
   cargo install --path w600-flash --force
   ```

3. 编译并烧录示例。`examples-w800` 里的 `rust-toolchain.toml` 会让 rustup 自动安装所需的 nightly 和 `rust-src`：

   ```powershell
   cd examples-w800
   $env:W600_PORT = "COM6"          # Linux: export W600_PORT=/dev/ttyUSB0
   cargo run --release --bin blinky -- --monitor
   ```

   不设置 `W600_PORT` 时只生成镜像（`target/csky-none-elfabiv2hf/release/blinky.img`，以及含 secboot 的 `blinky.fls`）。
   与官方 `make flash` 相同，烧录的是 `.fls`：芯片上的 secboot 与 SDK 不匹配时，`wm_tool` 会拒绝只含应用的 `.img`。
   默认通过 RTS 复位芯片；开发板不支持时加上 `--reset none`，在工具提示同步时手动按复位键。
   烧录完成后程序会立即运行，`--monitor` 切换到 115200 波特率之前的输出会丢失，需要看启动日志时按一下复位键。

示例的 LED 使用 PB0 / PB1 / PB2（低电平点亮），适用于 TB-08 开发板和海凌科 HLK-W800-KIT（需打开板上的 LED 拨码开关），
其他开发板请按原理图修改。除了与 W600 相同的示例，还有 `ble`：以 `W800-Rust` 为名广播，
用手机蓝牙调试工具（nRF Connect、LightBlue 等）连接后，在服务 `0xFFE0` 的特征 `0xFFE1` 上订阅通知并写入数据，
串口会打印收到的内容，并通过同一个特征回复 `ack: <收到的内容>`。新项目复制 `examples-w800` 目录即可，需要的文件是 `.cargo/config.toml`、
`rust-toolchain.toml` 和 `csky-none-elfabiv2hf.json`。

### 与 W600 的差异

- **构建**：使用自定义目标 `csky-none-elfabiv2hf.json`（`ck804ef`，硬件浮点 ABI），由 nightly 的 `build-std` 编译 `core` / `alloc`，
  链接器为 `csky-elfabiv2-gcc`，C 库为工具链自带的 minilibc。**不能使用 `opt-level = 0`**：LLVM 的 C-SKY 后端
  在不优化时无法编译 `compiler_builtins`。
- **原子操作**：LLVM 把 C-SKY 上的原子操作全部编译成 `__atomic_*` 库函数调用，工具链又没有 libatomic，
  因此由 `w600` crate 以关中断的方式实现（W800 是单核）。
- **串口**：有 UART1 ～ UART5。SDK 默认开启 AT 指令，启动时把 **UART1（PB6 / PB7）用作 AT 指令口**；
  `Uart::new` 打开 UART1 会接管它，但建议使用其他串口（示例用 UART4：PB4 / PB5）。
- **PWM**：通道 0 ～ 4（`pwm0` ～ `pwm4`）对应 SDK 的 `wm_pwm0_config` ～ `wm_pwm4_config`。
- **ADC**：`read_millivolts` 由 SDK 直接换算，通道 0 ～ 3 依次对应 PA1、PA4、PA3、PA2；没有 `read_raw`。
- **蓝牙**：`w600::ble` 提供 BLE 从机：广播设备名，接收主机写入的数据，通过 notify 向主机发送数据，以及连接 / 断开事件。
  它直接使用 SDK 自带的 NimBLE 协议栈，注册与 HM-10 串口透传模块相同的服务：服务 `0xFFE0`，特征 `0xFFE1` 同时支持
  Read / Write / Write Without Response / Notify。单条通知最长为 ATT MTU - 3 字节（默认 20，主机协商更大的 MTU 后增大）。
- **Flash 用户区共 104KB**（0x81E0000 起）。应用代码区约 1087KB，SDK 默认配置（含 AT 指令、蓝牙）下示例约占 700KB；
  堆约 166KB。
- 没有 W600 的“任务栈必须位于 0x20028000 以下”和“PB31 不支持中断”限制；其余限制（优先级范围、2ms 节拍、
  同时最多 8 个 socket 等）相同。

### 维护：更新 W800 的静态库和绑定

```sh
cargo xtask w800-all   # 等价于 cargo xtask w800-sdk && cargo xtask w800-bindgen
```

W800 SDK 自带按官方默认配置（NimBLE 蓝牙、AT 指令）编译好的各模块静态库，`xtask` 不重新编译 SDK，而是：

1. 从联盛德官网下载 `W80X_SDK_v1.00.10.rar` 并校验 SHA-256，用 bsdtar 解压（Windows 自带；Linux 需安装 `libarchive-tools`），
   也可以用 `W800_SDK_DIR` 指向已解压的 SDK；
2. 把 `lib/w800` 下的库去掉调试信息（约 30MB → 5MB）后复制到 `w800-sys/lib`，并复制 `w800_secboot.img`、`wm_tool` 和 zlib 源码；
3. 用 bindgen（`--target=csky-unknown-elf`）从 `w800-sys/wrapper.h` 生成 `w800-sys/src/bindings.rs`。

需要 C-SKY 工具链（在 `PATH` 中，或用 `W800_TOOLCHAIN` 指定 `bin` 目录）和 libclang。同一环境下重复执行得到的文件完全相同。
需要注意的细节：

- SDK 库必须以 `--whole-archive` 方式链接（与官方 Makefile 相同）：中断处理函数以强符号覆盖启动文件中的弱符号，
  没有其他地方引用它们；`w800-sys` 的 build.rs 已经这样处理；
- minilibc 的 `stddef.h` 等会 `#include_next` 编译器自带的同名头文件，bindgen 的头文件搜索顺序必须是 minilibc 在前；
- C-SKY 的 `char` 是无符号的，枚举为 4 字节，64 位类型按 4 字节对齐；minilibc 的 `timeval.tv_usec` 是 64 位；
- lwIP 2.1 的 socket 编号从 1 开始（`LWIP_SOCKET_OFFSET`），`closesocket` / `gethostbyname` 是宏，
  `w800-sys` 以 W600 的名字重新导出了对应的函数；
- rustc 内置的 LLVM 汇编器不支持 `mfcr` / `psrclr` 的 `psr`、`ie` 操作数，开关中断的指令以机器码形式写在 `w600/src/atomic.rs` 中。

## 当前状态

- W600：所有示例都能在 stable Rust 下用 rust-lld 编译、链接（Windows 上验证；烧录工具在 Linux 上也验证过），
  生成的镜像与官方工具链产物格式一致。`blinky`、`threads`、`tcp_server`、`tcp_client`、`https_client`、`servo`
  已在真实硬件上运行通过。
- W800：所有示例都能用 nightly + `csky-elfabiv2-gcc` 编译、链接（Windows 上验证，烧录工具在 Linux 上也验证过），
  `blinky`、`threads`、`tcp_server`、`tcp_client`、`https_client`、`servo` 和 `ble`（广播、连接、在同一特征上接收写入并回复通知）
  已在 TB-08 开发板上运行通过。
  镜像头与官方固件格式一致，`.fls` 中的 secboot 与官方固件相同。LLVM 与 GCC 之间的调用约定（整数 / 浮点 / 64 位参数、
  结构体传值与返回）和所用结构体的布局已逐项对照 GCC 的输出核对。

**两款芯片的 `peripherals` 还没有在真实硬件上测试过**；`https_client` 只测试了不校验证书的情况
（校验证书的逻辑只在 PC 上验证过）。欢迎反馈问题。

## 许可

Rust 代码以 MIT 或 Apache-2.0 授权。`w600-sys/bin`、`w800-sys/bin`、`tools/wm_tool`、`tools/wm_tool_w800` 以及
`w600-sys/lib`、`w800-sys/lib` 中的 SDK 库（`libw600sdk.a`、`libwlan.a`、`libairkiss_log.a` 等）来自联盛德官方 SDK，
版权归北京联盛德微电子有限责任公司所有；`w800.x` 基于 C-SKY 的 Apache-2.0 链接脚本；`tools/wm_tool_w800/zlib-1.2.11`
为 zlib（zlib 许可）；`libc.a`、`libm.a` 来自 newlib（BSD 类许可），`libgcc.a` 来自 GCC（GPL 附运行库例外，允许链接进任意程序）。
TLS 使用的 embedded-tls（`third_party/embedded-tls`，Apache-2.0）及 RustCrypto 系列 crate 以 Apache-2.0 / MIT 授权。
