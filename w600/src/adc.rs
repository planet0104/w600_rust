//! ADC：带自动校准的单次采样、芯片内部温度。采样期间忙等，单次耗时约数百微秒。

use crate::sys;

/// ADC 外设的所有权。
pub struct ADC {
    _private: (),
}

impl ADC {
    pub(crate) unsafe fn steal() -> Self {
        Self { _private: () }
    }
}

pub struct Adc {
    _adc: ADC,
}

impl Adc {
    pub fn new(adc: ADC) -> Self {
        Self { _adc: adc }
    }

    /// 读取通道 `channel` 的校准后原始值（14 位，8192 对应约 1.584V），仅 W600。
    /// 通道与引脚的对应关系见数据手册，调用时会自动把对应引脚切换为模拟输入。
    #[cfg(target_arch = "arm")]
    pub fn read_raw(&mut self, channel: u8) -> u16 {
        unsafe {
            sys::wm_adc_config(channel);
            sys::adc_get_inputVolt(channel)
        }
    }

    /// 读取通道 `channel` 的电压（毫伏），调用时会自动把对应引脚切换为模拟输入。
    ///
    /// W600 的换算公式取自 SDK：`(raw - 8192) / 8192 * 2.25 / 1.2 + 1.584`。
    /// W800 由 SDK 直接换算：通道 0 ～ 3 为单端输入，依次对应 PA1、PA4、PA3、PA2；
    /// 8、9 为差分输入，SDK 不会自动配置它们的引脚。
    pub fn read_millivolts(&mut self, channel: u8) -> i32 {
        #[cfg(target_arch = "arm")]
        {
            let raw = self.read_raw(channel) as i32;
            (raw - 8192) * 2250 * 10 / (8192 * 12) + 1584
        }
        #[cfg(target_arch = "csky")]
        unsafe {
            sys::wm_adc_config(channel);
            sys::adc_get_inputVolt(channel)
        }
    }

    /// 芯片内部温度（千分之一摄氏度）。
    pub fn temperature_millicelsius(&mut self) -> i32 {
        unsafe { sys::adc_get_interTemp() as i32 }
    }
}
