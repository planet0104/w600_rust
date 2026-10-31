//! GPIO：推挽输出、带上下拉的输入、引脚中断。实现 `embedded-hal` 的数字 IO trait。

use core::convert::Infallible;
use core::ffi::c_void;

use alloc::boxed::Box;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};

use crate::sys::{self, tls_gpio_attr, tls_gpio_dir, tls_gpio_irq_trig, tls_io_name};

/// 一个引脚的所有权。通过 [`Peripherals::take`](crate::Peripherals::take) 获得，交给驱动后即被占用。
#[derive(Debug)]
pub struct Pin {
    id: u8,
}

impl Pin {
    /// SDK 中的引脚编号（`WM_IO_PA_00` = 0 … `WM_IO_PB_31` = 47）。
    pub fn id(&self) -> u8 {
        self.id
    }

    pub(crate) fn io(&self) -> tls_io_name {
        tls_io_name(self.id)
    }

    /// # Safety
    ///
    /// 调用者需保证该引脚没有被其他驱动使用。
    pub unsafe fn steal(id: u8) -> Pin {
        assert!(id <= 47, "invalid pin id");
        Pin { id }
    }
}

macro_rules! pins {
    ($($name:ident = $id:expr),* $(,)?) => {
        /// 芯片的全部引脚。不同封装实际引出的引脚不同，请对照模块的数据手册。
        #[derive(Debug)]
        pub struct Pins {
            $(pub $name: Pin,)*
        }

        impl Pins {
            pub(crate) unsafe fn steal() -> Self {
                Self { $($name: Pin { id: $id },)* }
            }
        }
    };
}

pins!(
    pa0 = 0, pa1 = 1, pa2 = 2, pa3 = 3, pa4 = 4, pa5 = 5, pa6 = 6, pa7 = 7,
    pa8 = 8, pa9 = 9, pa10 = 10, pa11 = 11, pa12 = 12, pa13 = 13, pa14 = 14, pa15 = 15,
    pb0 = 16, pb1 = 17, pb2 = 18, pb3 = 19, pb4 = 20, pb5 = 21, pb6 = 22, pb7 = 23,
    pb8 = 24, pb9 = 25, pb10 = 26, pb11 = 27, pb12 = 28, pb13 = 29, pb14 = 30, pb15 = 31,
    pb16 = 32, pb17 = 33, pb18 = 34, pb19 = 35, pb20 = 36, pb21 = 37, pb22 = 38, pb23 = 39,
    pb24 = 40, pb25 = 41, pb26 = 42, pb27 = 43, pb28 = 44, pb29 = 45, pb30 = 46, pb31 = 47,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Low,
    High,
}

impl From<bool> for Level {
    fn from(high: bool) -> Self {
        if high { Level::High } else { Level::Low }
    }
}

impl From<Level> for bool {
    fn from(level: Level) -> bool {
        level == Level::High
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pull {
    None,
    Up,
    Down,
}

/// 中断触发方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    RisingEdge,
    FallingEdge,
    AnyEdge,
    HighLevel,
    LowLevel,
}

/// 推挽输出。
pub struct Output {
    pin: Pin,
}

impl Output {
    pub fn new(pin: Pin, initial: Level) -> Self {
        unsafe {
            sys::tls_gpio_cfg(pin.io(), tls_gpio_dir::WM_GPIO_DIR_OUTPUT, tls_gpio_attr::WM_GPIO_ATTR_FLOATING);
            sys::tls_gpio_write(pin.io(), initial as u8);
        }
        Self { pin }
    }

    pub fn set_level(&mut self, level: Level) {
        unsafe { sys::tls_gpio_write(self.pin.io(), level as u8) };
    }

    pub fn set_high(&mut self) {
        self.set_level(Level::High);
    }

    pub fn set_low(&mut self) {
        self.set_level(Level::Low);
    }

    /// 当前输出电平。
    pub fn level(&self) -> Level {
        (unsafe { sys::tls_gpio_read(self.pin.io()) } != 0).into()
    }

    pub fn toggle(&mut self) {
        let next = if self.level() == Level::High { Level::Low } else { Level::High };
        self.set_level(next);
    }

    /// 释放引脚。
    pub fn into_pin(self) -> Pin {
        self.pin
    }
}

/// 输入，可选上下拉，可注册中断回调。
pub struct Input {
    pin: Pin,
    irq: Option<*mut IrqContext>,
}

// 中断上下文只在 IRQ 关闭后才会被释放，裸指针本身可以安全地跨线程移动。
unsafe impl Send for Input {}

struct IrqContext {
    pin: tls_io_name,
    callback: Box<dyn FnMut() + Send>,
}

unsafe extern "C" fn irq_trampoline(arg: *mut c_void) {
    let ctx = unsafe { &mut *arg.cast::<IrqContext>() };
    unsafe { sys::tls_clr_gpio_irq_status(ctx.pin) };
    (ctx.callback)();
}

impl Input {
    pub fn new(pin: Pin, pull: Pull) -> Self {
        let attr = match pull {
            Pull::None => tls_gpio_attr::WM_GPIO_ATTR_FLOATING,
            Pull::Up => tls_gpio_attr::WM_GPIO_ATTR_PULLHIGH,
            Pull::Down => tls_gpio_attr::WM_GPIO_ATTR_PULLLOW,
        };
        unsafe { sys::tls_gpio_cfg(pin.io(), tls_gpio_dir::WM_GPIO_DIR_INPUT, attr) };
        Self { pin, irq: None }
    }

    pub fn level(&self) -> Level {
        (unsafe { sys::tls_gpio_read(self.pin.io()) } != 0).into()
    }

    pub fn is_high(&self) -> bool {
        self.level() == Level::High
    }

    pub fn is_low(&self) -> bool {
        self.level() == Level::Low
    }

    /// 注册中断回调（替换已有的回调）。PB31 不支持中断。
    ///
    /// 回调运行在**中断上下文**：不能阻塞、不能加锁、不要分配内存；
    /// 可以用 [`Channel::send`](crate::sync::Channel::send) 或
    /// [`Semaphore::release`](crate::sync::Semaphore::release) 通知线程。
    pub fn on_interrupt(&mut self, trigger: Trigger, callback: impl FnMut() + Send + 'static) {
        assert!(self.pin.id < 47, "PB31 does not support interrupts");
        self.disable_interrupt();
        let ctx = Box::into_raw(Box::new(IrqContext { pin: self.pin.io(), callback: Box::new(callback) }));
        self.irq = Some(ctx);
        let trig = match trigger {
            Trigger::RisingEdge => tls_gpio_irq_trig::WM_GPIO_IRQ_TRIG_RISING_EDGE,
            Trigger::FallingEdge => tls_gpio_irq_trig::WM_GPIO_IRQ_TRIG_FALLING_EDGE,
            Trigger::AnyEdge => tls_gpio_irq_trig::WM_GPIO_IRQ_TRIG_DOUBLE_EDGE,
            Trigger::HighLevel => tls_gpio_irq_trig::WM_GPIO_IRQ_TRIG_HIGH_LEVEL,
            Trigger::LowLevel => tls_gpio_irq_trig::WM_GPIO_IRQ_TRIG_LOW_LEVEL,
        };
        unsafe {
            sys::tls_gpio_isr_register(self.pin.io(), Some(irq_trampoline), ctx.cast());
            sys::tls_gpio_irq_enable(self.pin.io(), trig);
        }
    }

    pub fn disable_interrupt(&mut self) {
        if let Some(ctx) = self.irq.take() {
            unsafe {
                sys::tls_gpio_irq_disable(self.pin.io());
                sys::tls_gpio_isr_register(self.pin.io(), None, core::ptr::null_mut());
                drop(Box::from_raw(ctx));
            }
        }
    }

    /// 释放引脚（同时关闭中断）。
    pub fn into_pin(mut self) -> Pin {
        self.disable_interrupt();
        let pin = Pin { id: self.pin.id };
        core::mem::forget(self);
        pin
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        self.disable_interrupt();
    }
}

impl ErrorType for Output {
    type Error = Infallible;
}

impl OutputPin for Output {
    fn set_low(&mut self) -> Result<(), Infallible> {
        Output::set_low(self);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Infallible> {
        Output::set_high(self);
        Ok(())
    }
}

impl StatefulOutputPin for Output {
    fn is_set_high(&mut self) -> Result<bool, Infallible> {
        Ok(self.level() == Level::High)
    }

    fn is_set_low(&mut self) -> Result<bool, Infallible> {
        Ok(self.level() == Level::Low)
    }
}

impl ErrorType for Input {
    type Error = Infallible;
}

impl InputPin for Input {
    fn is_high(&mut self) -> Result<bool, Infallible> {
        Ok(Input::is_high(self))
    }

    fn is_low(&mut self) -> Result<bool, Infallible> {
        Ok(Input::is_low(self))
    }
}
