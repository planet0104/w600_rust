//! 联盛德 W600 / W800 的 Rust 开发库。
//!
//! 芯片由编译目标决定：`thumbv7m-none-eabi` 对应 W600（[`sys`] 为 `w600-sys`），
//! C-SKY 目标对应 W800（[`sys`] 为 `w800-sys`）。两者的接口相同，个别差异在各模块文档中注明。
//!
//! 底层复用官方 SDK（FreeRTOS、Wi-Fi 协议栈、lwIP、外设驱动），在其上提供符合 Rust 习惯的接口：
//!
//! - 运行时：[`entry!`] 入口宏、基于 SDK 堆的全局分配器、[`println!`]、panic 处理
//! - RTOS：[`thread`]（闭包线程、join）、[`sync`]（`Mutex`、`Semaphore`、`Channel`）、[`time`]
//! - 外设：[`gpio`]、[`pwm`]、[`uart`]、[`i2c`]、[`spi`]、[`adc`]、[`timer`]、[`flash`]，
//!   并实现 `embedded-hal` 1.0 / `embedded-io` 0.7 的 trait
//! - 网络：[`wifi`]（STA 联网）、[`net`]（TCP/UDP socket、DNS）、[`tls`]（TLS 1.3 客户端，用于 HTTPS）
//! - 蓝牙（仅 W800）：`ble`（BLE 从机广播、接收主机写入的数据）
//!
//! ```no_run
//! #![no_std]
//! #![no_main]
//!
//! use w600::gpio::{Level, Output};
//! use w600::{println, thread};
//! use core::time::Duration;
//!
//! w600::entry!(main);
//!
//! fn main() {
//!     let p = w600::Peripherals::take().unwrap();
//!     let mut led = Output::new(p.pins.pb16, Level::High);
//!     loop {
//!         led.toggle();
//!         println!("tick");
//!         thread::sleep(Duration::from_millis(500));
//!     }
//! }
//! ```

#![no_std]
#![cfg_attr(target_arch = "csky", feature(asm_experimental_arch))]

extern crate alloc;

pub use embedded_hal;
pub use embedded_io;
#[cfg(target_arch = "arm")]
pub use w600_sys as sys;
#[cfg(target_arch = "csky")]
pub use w800_sys as sys;

#[cfg(target_arch = "csky")]
mod atomic;
mod heap;
mod peripherals;
#[macro_use]
mod print;
#[doc(hidden)]
pub mod rt;

pub mod adc;
#[cfg(target_arch = "csky")]
pub mod ble;
pub mod delay;
pub mod flash;
pub mod gpio;
pub mod i2c;
pub mod net;
pub mod pwm;
pub mod spi;
pub mod sync;
pub mod system;
pub mod thread;
pub mod time;
pub mod timer;
pub mod tls;
pub mod uart;
pub mod wifi;

pub use peripherals::Peripherals;

/// 当前是否运行在中断上下文中。
pub fn in_interrupt() -> bool {
    unsafe { sys::tls_get_isr_count() > 0 }
}
