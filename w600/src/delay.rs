//! 实现 `embedded-hal` 的 `DelayNs`，供需要延时的驱动使用。

use core::time::Duration;

use embedded_hal::delay::DelayNs;

use crate::sys;

/// 不足 10ms 的延时用忙等实现（精确但占用 CPU），更长的延时让出 CPU 给其他线程。
#[derive(Debug, Clone, Copy, Default)]
pub struct Delay;

const SLEEP_THRESHOLD_NS: u64 = 10_000_000;

impl Delay {
    pub fn new() -> Self {
        Self
    }

    fn wait_ns(&mut self, ns: u64) {
        if ns >= SLEEP_THRESHOLD_NS && !crate::in_interrupt() {
            crate::thread::sleep(Duration::from_nanos(ns));
            return;
        }
        let mut clk = sys::tls_sys_clk::default();
        unsafe { sys::tls_sys_clk_get(&mut clk) };
        let cycles = ns * clk.cpuclk as u64 / 1000;
        spin(cycles.min(u32::MAX as u64) as u32);
    }
}

/// 至少忙等 `cycles` 个 CPU 周期（每次循环在 Cortex-M3 上约 3 个周期、在 XT804 上约 2 个周期，
/// 从 Flash 执行时会更慢）。
#[cfg(target_arch = "arm")]
fn spin(cycles: u32) {
    let iterations = cycles / 3 + 1;
    unsafe {
        core::arch::asm!(
            "1:",
            "subs {0}, #1",
            "bne 1b",
            inout(reg) iterations => _,
            options(nomem, nostack),
        );
    }
}

#[cfg(target_arch = "csky")]
fn spin(cycles: u32) {
    let iterations = cycles / 2 + 1;
    unsafe {
        core::arch::asm!(
            "1:",
            "subi32 {0}, {0}, 1",
            "bnez32 {0}, 1b",
            inout(reg) iterations => _,
            options(nomem, nostack),
        );
    }
}

impl DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        self.wait_ns(ns as u64);
    }

    fn delay_us(&mut self, us: u32) {
        self.wait_ns(us as u64 * 1_000);
    }

    fn delay_ms(&mut self, ms: u32) {
        self.wait_ns(ms as u64 * 1_000_000);
    }
}
