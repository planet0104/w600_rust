# lib_w600: Rust for WinnerMicro W600 / W800

[中文](README.zh-CN.md) | English

A Rust development library for the WinnerMicro W600 and W800 Wi-Fi (and BLE) microcontrollers, built on top of the
official SDKs (W600: WM_SDK_W60X_G3.04.00; W800: W80X_SDK_v1.00.10). FreeRTOS, the Wi-Fi stack, lwIP and the peripheral
drivers stay the official implementations; the Rust side provides idiomatic, safe APIs and implements
`embedded-hal` 1.0 / `embedded-io` 0.7, so community driver crates work out of the box.

- Both chips share the same `w600` crate and the same API. The chip is selected automatically by the compilation
  target; the few differences are noted in the documentation.
- **The W600 only needs stable Rust**: no SDK, ARM toolchain, libclang or make. The SDK and newlib are shipped as
  prebuilt static libraries, the FFI bindings are pre-generated with bindgen, and linking uses Rust's bundled rust-lld.
- **The W800's CPU is a T-Head XT804 (C-SKY architecture)**. Rust has no official bare-metal C-SKY target and rust-lld
  does not support C-SKY, so it needs nightly Rust (selected automatically by `examples-w800/rust-toolchain.toml`)
  and T-Head's `csky-elfabiv2-gcc` toolchain (used for linking only). See [W800](#w800).
- A single `cargo run` builds, packages and flashes over the serial port.

| Directory | Contents |
| --- | --- |
| `w600-sys` | W600: raw bindgen bindings, prebuilt static libraries (SDK + newlib), linker script `w600.x` |
| `w800-sys` | W800: raw bindgen bindings, prebuilt static libraries shipped with the SDK, linker script `w800.x` |
| `w600` | Runtime, threads and synchronization, GPIO/PWM/UART/I2C/SPI/ADC/timers/Flash, Wi-Fi, TCP/UDP (shared by both chips) |
| `w600-flash` | Packaging and flashing tool, used as the cargo runner; detects the chip from the ELF architecture |
| `examples` | W600 examples, also a template for new projects |
| `examples-w800` | W800 examples and template (including the C-SKY target spec `csky-none-elfabiv2hf.json`) |
| `xtask` | Maintainer tool: downloads the SDKs and regenerates the static libraries and bindings |
| `third_party/embedded-tls` | The TLS 1.3 library embedded-tls 0.19.0 with one compatibility fix and one stack-usage fix |

## Quick start (W600)

1. Install Rust (stable) and the compilation target:

   ```sh
   rustup target add thumbv7m-none-eabi
   ```

2. Install the flashing tool:

   ```sh
   cargo install --path w600-flash
   ```

   It bundles the official `wm_tool` for both the W600 and the W800. On Windows it uses the `wm_tool.exe` shipped with
   the SDK; on Linux / macOS the installer compiles `wm_tool.c` under `tools/` (the W800 version together with the
   bundled zlib) with the local C compiler (gcc / clang, usually preinstalled).

3. Connect the board, then build and flash an example:

   ```powershell
   cd examples
   $env:W600_PORT = "COM6"          # Linux: export W600_PORT=/dev/ttyUSB0
   cargo run --release --bin blinky -- --monitor
   ```

   Without `W600_PORT`, only the image is generated (`target/thumbv7m-none-eabi/release/blinky_gz.img`).
   If the board's RTS line is not wired to the reset pin, add `--reset none` and press the reset button when the tool
   asks for synchronization.

For a new project, copy the `examples` directory and change the `path` dependencies in `Cargo.toml` to the actual
locations. The essential part is these lines in `.cargo/config.toml`:

```toml
[build]
target = "thumbv7m-none-eabi"

[target.thumbv7m-none-eabi]
runner = "w600-flash"
rustflags = ["-C", "link-arg=-Tw600.x"]
```

If you prefer GNU ld, set `linker = "arm-none-eabi-gcc"` and add `-nostdlib`, `-mcpu=cortex-m3` and `-mthumb` to
rustflags (each passed as `link-arg=`). The linker script works with both.

## Example

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

More examples in `examples/src/bin` (and `examples-w800/src/bin`): `threads` (threads, locks, channels),
`tcp_server` (multi-threaded echo server), `tcp_client`, `https_client` (HTTPS over TLS 1.3),
`peripherals` (PWM breathing LED, button interrupt, UART echo, ADC temperature, Flash read/write),
`servo` (servo signal on PB15 for the W600 / PB3 for the W800: moves to a few fixed angles, then sweeps back and forth)
and `buzzer` (plays a loop of children's songs on a passive buzzer, including Happy Birthday, It's a Small World and
London Bridge Is Falling Down; same pins as `servo`). The W600 also has `music_box`, a music box for the TW-01
module: the button held at power-on decides which way a continuous-rotation servo turns while the songs play, and
after a power cycle it continues with the next song. See `examples/src/bin/music_box/README.md` (in Chinese) for
wiring and how to modify the servo.
Each PWM channel can only use a fixed set of pins (see the documentation of `Pwm::new`); `Pwm::new` returns
`InvalidPin` for any other pin.

## HTTPS / TLS

`w600::tls::TlsStream` is a TLS 1.3 client based on [embedded-tls](https://crates.io/crates/embedded-tls) that works on
both chips. It runs on top of `net::TcpStream` and also implements the `embedded-io` `Read` / `Write` traits:

```rust
let mut buffers = w600::tls::Buffers::new();                       // ~20KB record buffers, reusable
let mut s = w600::tls::TlsStream::connect("www.nmc.cn", 443, None, &mut buffers)?;
s.write_all(b"GET /rest/weather?stationid=2011 HTTP/1.1\r\nHost: www.nmc.cn\r\nConnection: close\r\n\r\n")?;
s.flush()?;                                                          // written data is sent on flush
```

- TLS 1.3 only (TLS_AES_128_GCM_SHA256, P-256 key exchange); SNI is sent automatically. Servers that only support
  TLS 1.2 (e.g. www.baidu.com) cannot be reached; the handshake fails with `HandshakeAborted(Fatal, ProtocolVersion)`.
- embedded-tls 0.19.0 fails the handshake (`DecodeError`) when the server lists a key exchange group it does not know
  (e.g. brainpool or the Chinese SM2), which many Chinese websites do. The copy in `third_party/embedded-tls` contains
  the fix (identical to the fix on the upstream main branch). It also marks the certificate verification functions
  `#[inline(never)]`; otherwise the verification code is inlined into the handshake function, which then reserves
  about 20KB of extra stack even when no certificate is verified.
- With `None` as the third argument the **server certificate is not verified**: the traffic is encrypted, but a
  man-in-the-middle attack cannot be prevented. With a DER-encoded root certificate, the certificate chain and the
  host name are verified (there is no real-time clock, so validity periods are not checked). Supported signature
  algorithms for verification: ECDSA P-256 and RSA on the W800; only ECDSA P-256 on the W600 because of its 512KB
  application area.
- Each connection uses about 30KB of heap. The handshake needs a large thread stack: 24KB is enough without
  certificate verification, about 48KB with it (the example picks the size based on whether `CA` is set). If the stack
  is too small the chip simply hangs without any error message. Code size is about 100KB (W600) / 200KB (W800).
- Randomness: the W800 uses its hardware true random number generator. The W600 has none; its seed comes from the
  noise of the on-chip temperature ADC and CPU timing jitter, which is weaker.

## Limitations to be aware of

These limitations come from the SDK or the chip itself. The library already deals with them, but you need to know
about them when writing code. The following applies to the W600; for W800 differences see [W800](#w800):

- **The entry point runs in a thread**: the SDK requires `UserMain` to return quickly, so `entry!` runs your `main` in a
  new thread (8KB stack by default), where infinite loops are fine.
- **Threads are never deleted**: the SDK's FreeRTOS frees task stacks to the wrong heap when deleting a task. Finished
  threads go into an idle pool and are reused by later `spawn` calls.
- **Lower priority numbers mean higher priority**: SDK internal tasks use 0–28, application threads may use 30–62,
  default 40. Threads with the same priority are time-sliced.
- **Task stacks must be below 0x20028000**, otherwise `spawn` returns an error. Stacks are allocated from the heap, so
  create long-running threads early at startup.
- **Interrupt callbacks** (GPIO interrupts, hardware timers) must not block or take locks; use `Channel::send` /
  `Semaphore::release` to notify a thread. Objects from `sync` must be used for the first time in a thread, or
  `init()` must be called first.
- **Time resolution is 2ms** (500Hz system tick); `sleep` and timeouts are rounded up.
- **Semaphore counts are limited to 5**, and **at most 8 sockets can be open at the same time** (SDK FreeRTOS / lwIP
  configuration).
- **UART0 is the debug port** used by `println!`; applications can use UART1 / UART2.
- **SPI chip select is controlled by hardware**: `SpiDevice` only supports the operation sequences `[Write]`, `[Read]`,
  `[Write, Read]` and `[Write, Write]`; full duplex is not supported.
- **The Flash user area is 44KB** (`flash::UserFlash`); the SDK erases automatically when writing.
- The heap is about 160KB, shared with the C code (`system::free_heap()` shows what is left). The application code area
  is 512KB; the Wi-Fi + TCP examples use about 330KB.
- The prebuilt library **disables the SDK's AT command subsystem** (`TLS_CONFIG_HOSTIF`), which saves about 150KB of
  Flash.

## Maintenance: regenerating the static library and bindings (W600)

```sh
cargo xtask all        # same as cargo xtask sdk && cargo xtask bindgen
```

`xtask` will:

1. Download the SDK from the WinnerMicro website and verify its SHA-256 (or use `W600_SDK_DIR` to point to an
   extracted or modified SDK);
2. Modify `Include/wm_config.h` (currently it only disables `TLS_CONFIG_HOSTIF`);
3. Parse each module's Makefile to get the source list and compile it in parallel by calling `arm-none-eabi-gcc`
   directly (no make or bash needed), archive it as `w600-sys/lib/libw600sdk.a`, and copy the closed-source
   `libwlan.a`, `libairkiss_log.a`, `secboot.img` and `wm_tool`, plus the Cortex-M3 soft-float `libc.a`, `libm.a` and
   `libgcc.a` from the toolchain (with debug info stripped);
4. Generate `w600-sys/src/bindings.rs` from `w600-sys/wrapper.h` with bindgen.

Maintainers need the [Arm GNU Toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (in `PATH`,
or point `W600_TOOLCHAIN` to its `bin` directory) and libclang (set `LIBCLANG_PATH`); users need neither. The current
libraries were built with Arm GNU Toolchain 15.2.Rel1, and rebuilding with the same version produces identical files.
Several easy-to-miss details are already handled by xtask:

- The SDK and the closed-source `wlan.a` are compiled with AAPCS **short enums** (`Tag_ABI_enum_size: small`), so
  bindgen must use `-fshort-enums`, otherwise struct layouts don't match;
- Socket error codes come from lwIP's own errno definitions, some of which differ from newlib's, so newlib's
  `<errno.h>` is not included;
- Compiling the old SDK with a recent GCC needs `-fcommon -fpermissive` and `__USE_W32_SOCKETS` to avoid newlib
  redefining `fd_set`;
- The startup file's `.stack` / `.heap` sections have no allocation flag; the original script relies on GNU ld's
  `(COPY)`, and rust-lld would place them at address 0. So `w600.x` reserves the main stack itself and defines the
  stack and heap symbols, and the startup file is compiled with `__HEAP_SIZE=0`;
- The startup file's interrupt vector table section `.isr_vector` has no allocation flag either. GNU ld still keeps the
  interrupt handlers referenced from the `KEEP` section, but rust-lld doesn't: `--gc-sections` removes `SVC_Handler`,
  `PendSV_Handler`, the peripheral interrupt handlers and so on, which are only referenced from the vector table. The
  corresponding vector entries become 0 and FreeRTOS crashes right at startup. So xtask adds the `"a"` flag to that
  section before compiling;
- `-ffile-prefix-map` replaces the absolute SDK path in `__FILE__` with the directory name, so libraries built on
  different machines and in different directories are identical;
- lwIP's `TCP_MSS` is changed from 1024 to 1460 and `TCP_WND` from 6 × MSS to 8 × MSS, the same as the W800 SDK. The
  front end of some servers (e.g. www.nmc.cn) stops sending when the advertised window drops below about 8KB, so with
  the original configuration the TLS handshake hangs. lwIP allocates its buffers from the heap on demand, so this does
  not increase static memory usage.

## W800

### Quick start

1. Install T-Head's C-SKY toolchain (800 series, minilibc variant) and add its `bin` directory to `PATH`:
   - Official download: [T-Head Open Chip Community](https://occ.t-head.cn/community/download?id=616214177698021376)
     (registration required). On Windows pick `csky-elfabiv2-tools-mingw-minilibc-*.tar.gz`, on Linux
     `csky-elfabiv2-tools-x86_64-minilibc-*.tar.gz`
   - This repository is verified with version `20210423`. If you cannot register, use the community
     [backup of the same version](https://github.com/crosstyan/Xuantie-800-gcc-elf-newlib-prebuilt/releases/tag/v0.00)

   On Windows you can install it into your user directory with the commands below (don't put it in the root of `C:\`,
   regular users have no write permission there):

   ```powershell
   $dir = "$env:USERPROFILE\csky-toolchain"
   New-Item -ItemType Directory -Force $dir | Out-Null
   curl.exe -L -o "$env:TEMP\csky.tar.gz" "https://github.com/crosstyan/Xuantie-800-gcc-elf-newlib-prebuilt/releases/download/v0.00/csky-elfabiv2-tools-mingw-minilibc-20210423.tar.gz"
   tar -xzf "$env:TEMP\csky.tar.gz" -C $dir
   # Add to the user PATH permanently (new terminals) and to the current terminal right away
   [Environment]::SetEnvironmentVariable("Path", "$dir\bin;" + [Environment]::GetEnvironmentVariable("Path", "User"), "User")
   $env:PATH = "$dir\bin;$env:PATH"
   csky-elfabiv2-gcc --version      # should print T-HEAD C-SKY Tools V3.10.29 ... B20210423
   ```

   On Linux, download `csky-elfabiv2-tools-x86_64-minilibc-20210423.tar.gz`, extract it and add `bin` to `PATH`.
   If you don't want to modify `PATH`, set `CARGO_TARGET_CSKY_NONE_ELFABIV2HF_LINKER` to the full path of
   `csky-elfabiv2-gcc` instead. A ``linker `csky-elfabiv2-gcc` not found`` error during the build means this step is
   not done.

2. Install the flashing tool (same as for the W600; reinstall if you installed it before, to get W800 support):

   ```sh
   cargo install --path w600-flash --force
   ```

3. Build and flash an example. The `rust-toolchain.toml` in `examples-w800` makes rustup install the required nightly
   and `rust-src` automatically:

   ```powershell
   cd examples-w800
   $env:W600_PORT = "COM6"          # Linux: export W600_PORT=/dev/ttyUSB0
   cargo run --release --bin blinky -- --monitor
   ```

   Without `W600_PORT`, only the images are generated (`target/csky-none-elfabiv2hf/release/blinky.img`, and
   `blinky.fls` including secboot). Like the official `make flash`, the `.fls` file is flashed: `wm_tool` rejects an
   application-only `.img` when the secboot on the chip doesn't match the SDK.
   The chip is reset through RTS by default; if the board doesn't support that, add `--reset none` and press the reset
   button when the tool asks for synchronization.
   The program starts right after flashing, so output printed before `--monitor` switches to 115200 baud is lost;
   press the reset button to see the boot log.

The example LEDs are on PB0 / PB1 / PB2 (active low), which matches the TB-08 board and the Hi-Link HLK-W800-KIT
(turn on the LED DIP switch on the board); for other boards, adjust according to the schematic. Besides the examples
shared with the W600 there is `ble`: it advertises as `W800-Rust`. Connect with a BLE debugging app on your phone
(nRF Connect, LightBlue, etc.), subscribe to notifications on characteristic `0xFFE1` of service `0xFFE0` and write
data to it: the serial console prints what was received, and the board replies `ack: <received data>` through the same
characteristic. For a new project, copy the `examples-w800` directory; the required files are `.cargo/config.toml`,
`rust-toolchain.toml` and `csky-none-elfabiv2hf.json`.

### Differences from the W600

- **Build**: uses the custom target `csky-none-elfabiv2hf.json` (`ck804ef`, hard-float ABI), with `core` / `alloc`
  compiled by nightly `build-std`; the linker is `csky-elfabiv2-gcc` and the C library is the toolchain's minilibc.
  **`opt-level = 0` cannot be used**: LLVM's C-SKY backend fails to compile `compiler_builtins` without optimization.
- **Atomics**: LLVM compiles all atomic operations on C-SKY into `__atomic_*` library calls, and the toolchain has no
  libatomic, so the `w600` crate implements them by disabling interrupts (the W800 is single-core).
- **UART**: UART1 – UART5 are available. The SDK enables AT commands by default and at startup **uses UART1 (PB6 / PB7)
  as the AT command port**; opening UART1 with `Uart::new` takes it over, but using another UART is recommended (the
  examples use UART4: PB4 / PB5).
- **PWM**: channels 0 – 4 (`pwm0` – `pwm4`) correspond to the SDK's `wm_pwm0_config` – `wm_pwm4_config`.
- **ADC**: `read_millivolts` is converted by the SDK directly; channels 0 – 3 map to PA1, PA4, PA3 and PA2. There is no
  `read_raw`.
- **Bluetooth**: `w600::ble` provides a BLE peripheral: advertising a device name, receiving data written by the
  central, sending data to the central via notifications, and connect / disconnect events. It uses the NimBLE stack
  shipped with the SDK and registers the same service as the HM-10 serial bridge module: service `0xFFE0`, with
  characteristic `0xFFE1` supporting Read / Write / Write Without Response / Notify. A single notification is at most
  ATT MTU - 3 bytes (20 by default, more after the central negotiates a larger MTU).
- **The Flash user area is 104KB** (starting at 0x81E0000). The application code area is about 1087KB; with the SDK's
  default configuration (AT commands and Bluetooth included) the examples use about 700KB. The heap is about 166KB.
- The W600 limitations "task stacks must be below 0x20028000" and "PB31 doesn't support interrupts" don't apply; the
  others (priority range, 2ms tick, at most 8 sockets, etc.) are the same.

### Maintenance: updating the W800 static libraries and bindings

```sh
cargo xtask w800-all   # same as cargo xtask w800-sdk && cargo xtask w800-bindgen
```

The W800 SDK ships static libraries for each module, prebuilt with the official default configuration (NimBLE
Bluetooth, AT commands). `xtask` does not recompile the SDK; instead it:

1. Downloads `W80X_SDK_v1.00.10.rar` from the WinnerMicro website, verifies its SHA-256 and extracts it with bsdtar
   (built into Windows; on Linux install `libarchive-tools`), or uses `W800_SDK_DIR` pointing to an extracted SDK;
2. Strips debug info from the libraries under `lib/w800` (about 30MB → 5MB), copies them to `w800-sys/lib`, and copies
   `w800_secboot.img`, `wm_tool` and the zlib sources;
3. Generates `w800-sys/src/bindings.rs` from `w800-sys/wrapper.h` with bindgen (`--target=csky-unknown-elf`).

This needs the C-SKY toolchain (in `PATH`, or point `W800_TOOLCHAIN` to its `bin` directory) and libclang. Running it
again in the same environment produces identical files. Details to be aware of:

- The SDK libraries must be linked with `--whole-archive` (as in the official Makefile): interrupt handlers override
  the weak symbols in the startup file with strong symbols and are not referenced from anywhere else; the `w800-sys`
  build.rs already does this;
- minilibc's `stddef.h` and others `#include_next` the compiler's headers of the same name, so bindgen's header search
  order must put minilibc first;
- On C-SKY, `char` is unsigned, enums are 4 bytes, 64-bit types are 4-byte aligned, and minilibc's `timeval.tv_usec`
  is 64-bit;
- lwIP 2.1 socket numbers start at 1 (`LWIP_SOCKET_OFFSET`), and `closesocket` / `gethostbyname` are macros;
  `w800-sys` re-exports the corresponding functions under the W600 names;
- The LLVM assembler built into rustc doesn't accept the `psr` / `ie` operands of `mfcr` / `psrclr`, so the
  instructions that disable and enable interrupts are written as machine code in `w600/src/atomic.rs`.

## Status

- W600: all examples compile and link with stable Rust and rust-lld (verified on Windows; the flashing tool is also
  verified on Linux), and the generated images have the same format as those from the official toolchain. `blinky`,
  `threads`, `tcp_server`, `tcp_client`, `https_client`, `servo` and `buzzer` have been run successfully on real
  hardware.
- W800: all examples compile and link with nightly + `csky-elfabiv2-gcc` (verified on Windows; the flashing tool is
  also verified on Linux). `blinky`, `threads`, `tcp_server`, `tcp_client`, `https_client`, `servo` and `ble`
  (advertising, connecting, receiving writes and replying with notifications on the same characteristic) have been
  run successfully on a TB-08 board. The image header matches the official firmware format, and the secboot in the
  `.fls` is identical to the official one. The calling convention between LLVM and GCC (integer / floating-point /
  64-bit arguments, passing and returning structs by value) and the layout of the structs used have been checked one
  by one against GCC's output.

**The `peripherals` example has not been tested on real hardware for either chip, nor have `buzzer` on the W800 and
`music_box` on the W600**, and
`https_client` has only been tested without certificate verification (the verification logic has only been verified on a PC). Bug reports are
welcome.

## License

The Rust code is licensed under MIT or Apache-2.0. `w600-sys/bin`, `w800-sys/bin`, `tools/wm_tool`,
`tools/wm_tool_w800` and the SDK libraries in `w600-sys/lib` and `w800-sys/lib` (`libw600sdk.a`, `libwlan.a`,
`libairkiss_log.a`, etc.) come from the official WinnerMicro SDKs and are copyrighted by Beijing WinnerMicro
Electronics Co., Ltd.; `w800.x` is based on C-SKY's Apache-2.0 linker script; `tools/wm_tool_w800/zlib-1.2.11` is zlib
(zlib license); `libc.a` and `libm.a` come from newlib (BSD-style licenses), and `libgcc.a` comes from GCC (GPL with the
runtime library exception, which allows linking into any program). embedded-tls (`third_party/embedded-tls`,
Apache-2.0) and the RustCrypto crates used for TLS are licensed under Apache-2.0 / MIT.
