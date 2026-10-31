use core::sync::atomic::{AtomicBool, Ordering};

use crate::{adc, gpio, i2c, pwm, spi, uart};

/// 所有外设的所有权。每个字段只能被取走一次，从而在编译期保证同一个引脚或外设不会被两处同时使用。
///
/// UART0 是调试串口（[`println!`](crate::println)），不在此列。
pub struct Peripherals {
    pub pins: gpio::Pins,
    pub uart1: uart::UART<1>,
    pub uart2: uart::UART<2>,
    #[cfg(target_arch = "csky")]
    pub uart3: uart::UART<3>,
    #[cfg(target_arch = "csky")]
    pub uart4: uart::UART<4>,
    #[cfg(target_arch = "csky")]
    pub uart5: uart::UART<5>,
    pub i2c: i2c::I2C,
    pub spi: spi::SPI,
    pub adc: adc::ADC,
    pub pwm0: pwm::PWM<0>,
    pub pwm1: pwm::PWM<1>,
    pub pwm2: pwm::PWM<2>,
    pub pwm3: pwm::PWM<3>,
    pub pwm4: pwm::PWM<4>,
}

static TAKEN: AtomicBool = AtomicBool::new(false);

impl Peripherals {
    /// 取得外设所有权，只有第一次调用返回 `Some`。
    pub fn take() -> Option<Self> {
        if TAKEN.swap(true, Ordering::AcqRel) {
            None
        } else {
            Some(unsafe { Self::steal() })
        }
    }

    /// 不检查是否已被取走。
    ///
    /// # Safety
    ///
    /// 调用者需保证不会出现两个驱动同时操作同一外设或引脚。
    pub unsafe fn steal() -> Self {
        unsafe {
            Self {
                pins: gpio::Pins::steal(),
                uart1: uart::UART::steal(),
                uart2: uart::UART::steal(),
                #[cfg(target_arch = "csky")]
                uart3: uart::UART::steal(),
                #[cfg(target_arch = "csky")]
                uart4: uart::UART::steal(),
                #[cfg(target_arch = "csky")]
                uart5: uart::UART::steal(),
                i2c: i2c::I2C::steal(),
                spi: spi::SPI::steal(),
                adc: adc::ADC::steal(),
                pwm0: pwm::PWM::steal(),
                pwm1: pwm::PWM::steal(),
                pwm2: pwm::PWM::steal(),
                pwm3: pwm::PWM::steal(),
                pwm4: pwm::PWM::steal(),
            }
        }
    }
}
