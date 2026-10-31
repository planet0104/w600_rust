//! 舵机测试：PB3 输出舵机控制信号，先依次转到 90°、0°、160°、90°，然后在 0°..160° 之间来回慢速扫动。
//!
//! 接线：舵机信号线（通常为橙色或黄色）接 PB3，电源线（红色）接外部 5V，地线（棕色或黑色）同时接外部电源地和开发板 GND。
//! 舵机转动时电流可达数百 mA 以上，不要直接用开发板的 3.3V 供电。
//!
//! 舵机由周期性脉冲控制，脉宽 0.5ms..2.5ms 对应 0°..180°（不同舵机略有差别，可调整 MIN_PULSE_US / MAX_PULSE_US）。
//! 有的舵机转不到 180°，到头时会顶住发出嗡嗡声，因此用 MAX_ANGLE 限制最大角度。
//! PWM 占空比只有 256 级：50Hz 时一级约 78µs，0°..180° 只有约 26 级（每级约 7°）。
//! 数字舵机通常支持更高的刷新频率，把 FREQ_HZ 改为 333 可得到约 170 级（每级约 1°）；模拟舵机（如 SG90）请保持 50Hz。

#![no_std]
#![no_main]

use core::time::Duration;

use w600::pwm::Pwm;
use w600::{println, thread};

const FREQ_HZ: u32 = 50;
const MIN_PULSE_US: u32 = 500;
const MAX_PULSE_US: u32 = 2500;
const MAX_ANGLE: u32 = 160;
const _: () = assert!(MAX_PULSE_US < 1_000_000 / FREQ_HZ, "脉宽必须小于 PWM 周期");

w600::entry!(main);

/// 脉宽对应的占空比（一个周期分为 256 级）。
fn duty_for_pulse(pulse_us: u32) -> u8 {
    ((pulse_us * FREQ_HZ * 256 + 500_000) / 1_000_000).min(255) as u8
}

/// 角度对应的占空比，超过 MAX_ANGLE 的按 MAX_ANGLE 处理。
fn duty_for_angle(angle: u32) -> u8 {
    duty_for_pulse(MIN_PULSE_US + (MAX_PULSE_US - MIN_PULSE_US) * angle.min(MAX_ANGLE) / 180)
}

fn main() {
    let p = w600::Peripherals::take().unwrap();
    let mut servo = Pwm::new(p.pwm3, p.pins.pb3, FREQ_HZ).unwrap();

    let steps = (duty_for_angle(MAX_ANGLE) - duty_for_angle(0)) as u32;
    let tenths = MAX_ANGLE * 10 / steps;
    println!(
        "servo on PB3, {FREQ_HZ} Hz, duty {}..{} ({steps} steps, about {}.{} degrees per step)",
        duty_for_angle(0),
        duty_for_angle(MAX_ANGLE),
        tenths / 10,
        tenths % 10
    );

    for angle in [90, 0, MAX_ANGLE, 90] {
        println!("angle {angle}, duty {}", duty_for_angle(angle));
        servo.set_duty(duty_for_angle(angle));
        thread::sleep(Duration::from_secs(1));
    }

    println!("sweeping 0..{MAX_ANGLE}");
    loop {
        for angle in (0..=MAX_ANGLE).chain((0..MAX_ANGLE).rev()) {
            servo.set_duty(duty_for_angle(angle));
            thread::sleep(Duration::from_millis(20));
        }
    }
}
