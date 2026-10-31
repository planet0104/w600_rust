//! 联盟微 W600 的 Rust 开发库。
//!
//! 底层复用官方 SDK（FreeRTOS、Wi-Fi 协议栈、lwIP、外设驱动），在其上提供符合 Rust 习惯的接口：
//!
//! - 运行时：[`entry!`] 入口宏、基于 SDK 堆的全局分配器、[`println!`]、panic 处理
//! - RTOS：[`thread`]（闭包线程、join）、[`sync`]（`Mutex`、`Semaphore`、`Channel`）、[`time`]
//! - 外设：[`gpio`]、[`pwm`]、[`uart`]、[`i2c`]、[`spi`]、[`adc`]、[`timer`]、[`flash`]，
//!   并实现 `embedded-hal` 1.0 / `embedded-io` 0.7 的 trait
//! - 网络：[`wifi`]（STA 联网）、[`net`]（TCP/UDP socket、DNS）
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

extern crate alloc;

pub use embedded_hal;
pub use embedded_io;
pub use w600_sys as sys;

mod heap;
mod peripherals;
#[macro_use]
mod print;
#[doc(hidden)]
pub mod rt;

pub mod adc;
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
pub mod uart;
pub mod wifi;

pub use peripherals::Peripherals;

/// 当前是否运行在中断上下文中。
pub fn in_interrupt() -> bool {
    unsafe { sys::tls_get_isr_count() > 0 }
}
