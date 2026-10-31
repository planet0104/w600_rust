use core::fmt::{self, Write};

use crate::sync::Mutex;
use crate::sys;

static STDOUT: Mutex<()> = Mutex::new(());

/// 调试串口（UART0，115200 8N1）。直接轮询写寄存器，不经过 newlib 的 printf，因而不分配内存。
pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            unsafe { sys::tls_uart_output_char(b as i32) };
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // 中断里不能等待互斥锁，直接输出（可能与线程的输出交错）。
    if crate::in_interrupt() {
        let _ = Stdout.write_fmt(args);
    } else {
        let _guard = STDOUT.lock();
        let _ = Stdout.write_fmt(args);
    }
}

/// 打印到调试串口（UART0）。
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::rt::_print(format_args!($($arg)*)) };
}

/// 打印到调试串口（UART0）并换行。
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::rt::_print(format_args!("{}\n", format_args!($($arg)*))) };
}
