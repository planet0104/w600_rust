//! PWM：5 个独立通道，占空比 0..=255。实现 `embedded-hal` 的 `SetDutyCycle`。

use core::convert::Infallible;

use embedded_hal::pwm::{ErrorType, SetDutyCycle};

use crate::gpio::Pin;
use crate::sys;

/// PWM 通道 `CH`（0..=4）的所有权。
pub struct PWM<const CH: u8> {
    _private: (),
}

impl<const CH: u8> PWM<CH> {
    pub(crate) unsafe fn steal() -> Self {
        Self { _private: () }
    }
}

/// 频率超出范围（APB 40MHz 时约 3Hz..=156kHz）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidFrequency;

/// 一路 PWM 输出。丢弃时停止输出。
pub struct Pwm {
    channel: u8,
    _pin: Pin,
}

fn check_frequency(hz: u32) -> Result<(), InvalidFrequency> {
    let mut clk = sys::tls_sys_clk::default();
    unsafe { sys::tls_sys_clk_get(&mut clk) };
    // SDK 计算 clkdiv = APB / 256 / freq，结果必须落在 1..=65535。
    let base = clk.apbclk * 1_000_000 / 256;
    if hz == 0 || base / hz == 0 || base / hz > u16::MAX as u32 {
        return Err(InvalidFrequency);
    }
    Ok(())
}

impl Pwm {
    /// 在 `pin` 上输出频率为 `freq_hz` 的 PWM，初始占空比为 0。
    pub fn new<const CH: u8>(_channel: PWM<CH>, pin: Pin, freq_hz: u32) -> Result<Self, InvalidFrequency> {
        check_frequency(freq_hz)?;
        unsafe {
            match CH {
                0 => sys::wm_pwm1_config(pin.io()),
                1 => sys::wm_pwm2_config(pin.io()),
                2 => sys::wm_pwm3_config(pin.io()),
                3 => sys::wm_pwm4_config(pin.io()),
                _ => sys::wm_pwm5_config(pin.io()),
            }
            if sys::tls_pwm_init(CH, freq_hz, 0, 0) != 0 {
                return Err(InvalidFrequency);
            }
            sys::tls_pwm_start(CH);
        }
        Ok(Self { channel: CH, _pin: pin })
    }

    /// 占空比，0 为常低，255 为常高。
    pub fn set_duty(&mut self, duty: u8) {
        unsafe { sys::tls_pwm_duty_set(self.channel, duty) };
    }

    pub fn set_frequency(&mut self, freq_hz: u32) -> Result<(), InvalidFrequency> {
        check_frequency(freq_hz)?;
        unsafe { sys::tls_pwm_freq_set(self.channel, freq_hz) };
        Ok(())
    }
}

impl Drop for Pwm {
    fn drop(&mut self) {
        unsafe { sys::tls_pwm_stop(self.channel) };
    }
}

impl ErrorType for Pwm {
    type Error = Infallible;
}

impl SetDutyCycle for Pwm {
    fn max_duty_cycle(&self) -> u16 {
        u8::MAX as u16
    }

    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Infallible> {
        self.set_duty(duty.min(u8::MAX as u16) as u8);
        Ok(())
    }
}
