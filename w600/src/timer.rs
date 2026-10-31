//! 硬件定时器（微秒精度）。回调运行在中断上下文。

use core::ffi::c_void;
use core::time::Duration;

use alloc::boxed::Box;

use crate::sys::{self, tls_timer_unit};

/// 没有空闲的硬件定时器。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoTimerAvailable;

type Callback = Box<dyn FnMut() + Send>;

/// 一个硬件定时器。创建后需要调用 [`start`](Timer::start)；丢弃时自动停止并释放。
pub struct Timer {
    id: u8,
    callback: *mut Callback,
}

unsafe impl Send for Timer {}

unsafe extern "C" fn trampoline(arg: *mut c_void) {
    let callback = unsafe { &mut *arg.cast::<Callback>() };
    callback();
}

fn split(d: Duration) -> (tls_timer_unit, u32) {
    match u32::try_from(d.as_micros()) {
        Ok(us) => (tls_timer_unit::TLS_TIMER_UNIT_US, us.max(1)),
        Err(_) => (tls_timer_unit::TLS_TIMER_UNIT_MS, d.as_millis().min(u32::MAX as u128) as u32),
    }
}

impl Timer {
    /// 每隔 `period` 调用一次 `callback`。
    ///
    /// 回调运行在**中断上下文**：不能阻塞、不能加锁、不要分配内存。
    pub fn periodic(period: Duration, callback: impl FnMut() + Send + 'static) -> Result<Self, NoTimerAvailable> {
        Self::create(period, true, Box::new(callback))
    }

    /// 启动后经过 `delay` 调用一次 `callback`。可以再次 `start` 重新计时。
    pub fn one_shot(delay: Duration, callback: impl FnMut() + Send + 'static) -> Result<Self, NoTimerAvailable> {
        Self::create(delay, false, Box::new(callback))
    }

    fn create(d: Duration, repeat: bool, callback: Callback) -> Result<Self, NoTimerAvailable> {
        let callback = Box::into_raw(Box::new(callback));
        let (unit, timeout) = split(d);
        let mut cfg = sys::tls_timer_cfg {
            unit,
            timeout,
            // W600 SDK 中是 u8，W800 SDK 中是 bool。
            is_repeat: repeat as _,
            callback: Some(trampoline),
            arg: callback.cast(),
        };
        let id = unsafe { sys::tls_timer_create(&mut cfg) };
        if id == sys::WM_TIMER_ID_INVALID as u8 {
            drop(unsafe { Box::from_raw(callback) });
            return Err(NoTimerAvailable);
        }
        Ok(Self { id, callback })
    }

    pub fn start(&self) {
        unsafe { sys::tls_timer_start(self.id) };
    }

    pub fn stop(&self) {
        unsafe { sys::tls_timer_stop(self.id) };
    }

    /// 修改周期（单位与创建时相同：创建时 ≤ 4294 秒按微秒计，否则按毫秒计）。
    pub fn set_period(&mut self, period: Duration) {
        let (_, value) = split(period);
        unsafe { sys::tls_timer_change(self.id, value) };
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        unsafe {
            sys::tls_timer_stop(self.id);
            sys::tls_timer_destroy(self.id);
            drop(Box::from_raw(self.callback));
        }
    }
}
