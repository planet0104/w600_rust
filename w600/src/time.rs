//! 系统时间。系统节拍为 500Hz，即精度 2ms；节拍计数约 99 天回绕一次。

use core::time::Duration;

use crate::sys;

/// 每秒的系统节拍数。
pub const TICK_HZ: u32 = sys::configTICK_RATE_HZ;
const MS_PER_TICK: u64 = 1000 / TICK_HZ as u64;

/// 单调时钟上的一个时刻，基于系统节拍。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u32);

impl Instant {
    pub fn now() -> Self {
        Instant(unsafe { sys::tls_os_get_time() })
    }

    /// 从 `earlier` 到 `self` 经过的时间。计数回绕后仍能正确计算（间隔小于约 99 天时）。
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration::from_millis(self.0.wrapping_sub(earlier.0) as u64 * MS_PER_TICK)
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }
}

/// 自启动以来经过的时间。
pub fn uptime() -> Duration {
    Instant(0).elapsed()
}

/// 把时长换算成节拍数，向上取整且至少为 1（SDK 中 0 表示永久等待）。
pub(crate) fn ticks(d: Duration) -> u32 {
    let ticks = d.as_millis().div_ceil(MS_PER_TICK as u128);
    ticks.clamp(1, u32::MAX as u128) as u32
}
