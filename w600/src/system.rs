//! 系统信息与控制：复位、MAC 地址、剩余内存、看门狗。

use core::time::Duration;

use crate::sys;

/// 软件复位芯片。
pub fn reset() -> ! {
    unsafe { sys::tls_sys_reset() };
    loop {
        core::hint::spin_loop();
    }
}

/// 出厂 MAC 地址。
pub fn mac_address() -> [u8; 6] {
    let mut mac = [0u8; 6];
    unsafe { sys::tls_get_mac_addr(mac.as_mut_ptr()) };
    mac
}

/// 堆的大致剩余字节数。
pub fn free_heap() -> usize {
    unsafe { sys::tls_mem_get_avail_heapsize() as usize }
}

/// CPU 主频（MHz）。
pub fn cpu_clock_mhz() -> u32 {
    let mut clk = sys::tls_sys_clk::default();
    unsafe { sys::tls_sys_clk_get(&mut clk) };
    clk.cpuclk
}

/// 启动看门狗。SDK 的空闲任务会自动喂狗，因此只有在 CPU 被持续占满、
/// 空闲任务超过 `timeout` 无法运行时才会复位。
pub fn start_watchdog(timeout: Duration) {
    let us = timeout.as_micros().min(u32::MAX as u128) as u32;
    unsafe { sys::tls_watchdog_init(us) };
}

/// 立即喂狗。
pub fn feed_watchdog() {
    unsafe { sys::tls_watchdog_clr() };
}
