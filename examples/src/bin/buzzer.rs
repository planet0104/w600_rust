//! 无源蜂鸣器播放儿歌《世上只有妈妈好》，播放完停 2 秒后重复。
//!
//! 接线：蜂鸣器模块的 I/O（信号）接 PB15，VCC 接 3.3V，GND 接 GND。也可以直接把无源蜂鸣器的两只脚接在 PB15 和 GND 之间，
//! 只是声音较小。无源蜂鸣器需要输入方波才会响（有源蜂鸣器通电就响，只能发出固定的一个音，不能用来放音乐）。
//!
//! 乐谱为 1=D、4/4 拍、每分钟 70 拍，整体升高一个八度：无源蜂鸣器在 1kHz 附近最响，原调的低音很轻。

#![no_std]
#![no_main]

use core::time::Duration;

use w600::pwm::Pwm;
use w600::{println, thread};

/// 每分钟的拍数（四分音符）。
const BPM: u32 = 70;
/// 每个音结尾的静音时长，让相邻的同音能分开。
const GAP_MS: u32 = 20;
/// 发声时的占空比（50% 方波）。
const ON_DUTY: u8 = 128;
/// 静音时的占空比（常低）。低电平触发的蜂鸣器模块（模块上的三极管为 PNP）请改为 255（常高）。
const OFF_DUTY: u8 = 0;

// 1=D 升高一个八度后各音的频率（Hz），前缀 L 为低音，M 为中音，H 为高音。
const L5: u32 = 440;
const L6: u32 = 494;
const M1: u32 = 587;
const M2: u32 = 659;
const M3: u32 = 740;
const M5: u32 = 880;
const M6: u32 = 988;
const H1: u32 = 1175;
const REST: u32 = 0;

/// （频率，时值），时值以八分音符为单位：1 = 八分音符，2 = 四分音符，3 = 附点四分音符，4 = 二分音符。
const MELODY: &[(u32, u32)] = &[
    // 世上只有妈妈好
    (M6, 3), (M5, 1), (M3, 2), (M5, 2),
    (H1, 2), (M6, 1), (M5, 1), (M6, 4),
    // 有妈的孩子像块宝
    (M3, 2), (M5, 1), (M6, 1), (M5, 2), (M3, 2),
    (M1, 1), (L6, 1), (M5, 1), (M3, 1), (M2, 4),
    // 投进妈妈的怀抱
    (M2, 3), (M3, 1), (M5, 2), (M5, 1), (M6, 1),
    (M3, 3), (M2, 1), (M1, 4),
    // 幸福享不了
    (M5, 3), (M3, 1), (M2, 1), (M1, 1), (L6, 1), (M1, 1),
    (L5, 6), (REST, 2),
];

w600::entry!(main);

fn sleep_ms(ms: u32) {
    thread::sleep(Duration::from_millis(ms as u64));
}

fn main() {
    let p = w600::Peripherals::take().unwrap();
    let mut buzzer = Pwm::new(p.pwm3, p.pins.pb15, M1).unwrap();
    buzzer.set_duty(OFF_DUTY);

    let eighth_ms = 60_000 / BPM / 2;
    loop {
        println!("playing: shi shang zhi you ma ma hao");
        for &(freq, eighths) in MELODY {
            let ms = eighths * eighth_ms;
            if freq == REST {
                sleep_ms(ms);
                continue;
            }
            buzzer.set_frequency(freq).unwrap();
            buzzer.set_duty(ON_DUTY);
            sleep_ms(ms - GAP_MS);
            buzzer.set_duty(OFF_DUTY);
            sleep_ms(GAP_MS);
        }
        sleep_ms(2000);
    }
}
