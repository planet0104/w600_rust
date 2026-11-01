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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 频率超出范围（APB 40MHz 时约 3Hz..=156kHz）。
    InvalidFrequency,
    /// 该引脚不能输出这个通道的 PWM，可用引脚见 [`Pwm::new`]。
    InvalidPin,
}

/// 各通道可以使用的引脚（与 SDK 的 `wm_pwmN_config` 一致）。
#[cfg(target_arch = "arm")]
const PINS: [&[sys::tls_io_name]; 5] = {
    use sys::tls_io_name as io;
    [
        &[io::WM_IO_PA_00, io::WM_IO_PB_18, io::WM_IO_PB_05, io::WM_IO_PA_05, io::WM_IO_PB_19, io::WM_IO_PB_30],
        &[io::WM_IO_PA_01, io::WM_IO_PB_17, io::WM_IO_PB_04, io::WM_IO_PA_07, io::WM_IO_PB_13, io::WM_IO_PB_20],
        &[io::WM_IO_PA_02, io::WM_IO_PB_16, io::WM_IO_PB_03, io::WM_IO_PA_08, io::WM_IO_PB_21],
        &[io::WM_IO_PA_03, io::WM_IO_PB_15, io::WM_IO_PB_02, io::WM_IO_PA_09, io::WM_IO_PB_22, io::WM_IO_PB_06],
        &[io::WM_IO_PA_04, io::WM_IO_PB_14, io::WM_IO_PB_01, io::WM_IO_PA_10, io::WM_IO_PB_23, io::WM_IO_PB_08],
    ]
};
#[cfg(target_arch = "csky")]
const PINS: [&[sys::tls_io_name]; 5] = {
    use sys::tls_io_name as io;
    [
        &[io::WM_IO_PB_00, io::WM_IO_PB_19, io::WM_IO_PB_12, io::WM_IO_PA_02, io::WM_IO_PA_10],
        &[io::WM_IO_PB_01, io::WM_IO_PB_20, io::WM_IO_PA_03, io::WM_IO_PA_11, io::WM_IO_PB_13],
        &[io::WM_IO_PA_00, io::WM_IO_PB_02, io::WM_IO_PA_12, io::WM_IO_PB_14, io::WM_IO_PB_24],
        &[io::WM_IO_PA_01, io::WM_IO_PB_03, io::WM_IO_PA_13, io::WM_IO_PB_15, io::WM_IO_PB_25],
        &[io::WM_IO_PA_04, io::WM_IO_PA_07, io::WM_IO_PA_14, io::WM_IO_PB_16, io::WM_IO_PB_26],
    ]
};

/// 一路 PWM 输出。丢弃时停止输出。
pub struct Pwm {
    channel: u8,
    _pin: Pin,
}

fn check_frequency(hz: u32) -> Result<(), Error> {
    let mut clk = sys::tls_sys_clk::default();
    unsafe { sys::tls_sys_clk_get(&mut clk) };
    // SDK 计算 clkdiv = APB / 256 / freq，结果必须落在 1..=65535。
    let base = clk.apbclk * 1_000_000 / 256;
    if hz == 0 || base / hz == 0 || base / hz > u16::MAX as u32 {
        return Err(Error::InvalidFrequency);
    }
    Ok(())
}

impl Pwm {
    /// 在 `pin` 上输出频率为 `freq_hz` 的 PWM，初始占空比为 0。
    ///
    /// 每个通道只能使用固定的几个引脚：
    ///
    /// | 通道 | W600 | W800 |
    /// |---|---|---|
    /// | `pwm0` | PA0、PA5、PB5、PB18、PB19、PB30 | PA2、PA10、PB0、PB12、PB19 |
    /// | `pwm1` | PA1、PA7、PB4、PB13、PB17、PB20 | PA3、PA11、PB1、PB13、PB20 |
    /// | `pwm2` | PA2、PA8、PB3、PB16、PB21 | PA0、PA12、PB2、PB14、PB24 |
    /// | `pwm3` | PA3、PA9、PB2、PB6、PB15、PB22 | PA1、PA13、PB3、PB15、PB25 |
    /// | `pwm4` | PA4、PA10、PB1、PB8、PB14、PB23 | PA4、PA7、PA14、PB16、PB26 |
    pub fn new<const CH: u8>(_channel: PWM<CH>, pin: Pin, freq_hz: u32) -> Result<Self, Error> {
        if !PINS[CH as usize].contains(&pin.io()) {
            return Err(Error::InvalidPin);
        }
        check_frequency(freq_hz)?;
        unsafe {
            // W600 SDK 的引脚配置函数从 1 开始编号，W800 从 0 开始。
            #[cfg(target_arch = "arm")]
            match CH {
                0 => sys::wm_pwm1_config(pin.io()),
                1 => sys::wm_pwm2_config(pin.io()),
                2 => sys::wm_pwm3_config(pin.io()),
                3 => sys::wm_pwm4_config(pin.io()),
                _ => sys::wm_pwm5_config(pin.io()),
            }
            #[cfg(target_arch = "csky")]
            match CH {
                0 => sys::wm_pwm0_config(pin.io()),
                1 => sys::wm_pwm1_config(pin.io()),
                2 => sys::wm_pwm2_config(pin.io()),
                3 => sys::wm_pwm3_config(pin.io()),
                _ => sys::wm_pwm4_config(pin.io()),
            }
            if sys::tls_pwm_init(CH, freq_hz, 0, 0) != 0 {
                return Err(Error::InvalidFrequency);
            }
        }
        let mut pwm = Self { channel: CH, _pin: pin };
        pwm.set_duty(0);
        unsafe { sys::tls_pwm_start(CH) };
        Ok(pwm)
    }

    /// 占空比，0 为常低，255 为常高。
    pub fn set_duty(&mut self, duty: u8) {
        // 硬件在比较值为 0 时每个周期仍输出 1 个计数的高电平（会让无源蜂鸣器发出轻微的声音），
        // 比较值 255 时固定为高。常低由反相输出加比较值 255 实现。
        unsafe {
            if duty == 0 {
                sys::tls_pwm_duty_set(self.channel, u8::MAX);
                sys::tls_pwm_out_inverse_cmd(self.channel, true as _);
            } else {
                sys::tls_pwm_out_inverse_cmd(self.channel, false as _);
                sys::tls_pwm_duty_set(self.channel, duty);
            }
        }
    }

    pub fn set_frequency(&mut self, freq_hz: u32) -> Result<(), Error> {
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
