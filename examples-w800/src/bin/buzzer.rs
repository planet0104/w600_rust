//! 无源蜂鸣器循环播放儿歌：《新年好》《两只老虎》《生日快乐》《小小世界》（主歌 + 副歌）《伦敦桥》，每首之间停 2 秒。
//!
//! 接线：蜂鸣器模块的 I/O（信号）接 PB3，VCC 接 3.3V，GND 接 GND。也可以直接把无源蜂鸣器的两只脚接在 PB3 和 GND 之间，
//! 只是声音较小。无源蜂鸣器需要输入方波才会响（有源蜂鸣器通电就响，只能发出固定的一个音，不能用来放音乐）。
//!
//! 乐谱都按 C 调整体升高一个八度：无源蜂鸣器在 1kHz 附近最响，低音很轻。

#![no_std]
#![no_main]

use core::time::Duration;

use w600::pwm::Pwm;
use w600::{println, thread};

/// 每个音结尾的静音时长，让相邻的同音能分开。
const GAP_MS: u32 = 20;
/// 发声时的占空比（50% 方波）。
const ON_DUTY: u8 = 128;
/// 静音时的占空比（常低）。低电平触发的蜂鸣器模块（模块上的三极管为 PNP）请改为 255（常高）。
const OFF_DUTY: u8 = 0;

// 各音的频率（Hz）。C5 即简谱 1=C 时升高一个八度的中音 1。
const G4: u32 = 392;
const A4: u32 = 440;
const B4: u32 = 494;
const C5: u32 = 523;
const D5: u32 = 587;
const E5: u32 = 659;
const F5: u32 = 698;
const G5: u32 = 784;
const A5: u32 = 880;
const B5: u32 = 988;
const C6: u32 = 1047;
const D6: u32 = 1175;
const E6: u32 = 1319;
const F6: u32 = 1397;
const G6: u32 = 1568;
const REST: u32 = 0;

/// 一首曲子。`notes` 为（频率，时值），时值以十六分音符为单位：
/// 1 = 十六分音符，2 = 八分音符，3 = 附点八分音符，4 = 四分音符，8 = 二分音符，12 = 附点二分音符。
struct Song {
    name: &'static str,
    /// 每分钟的拍数（四分音符）。
    bpm: u32,
    notes: &'static [(u32, u32)],
}

/// 每行一小节，开头单独一行的是弱起（不完整的第一小节）。
#[rustfmt::skip]
const SONGS: &[Song] = &[
    Song {
        name: "xin nian hao (Happy New Year)",
        bpm: 120,
        notes: &[
            (C5, 2), (C5, 2), (C5, 4), (G4, 4),
            (E5, 2), (E5, 2), (E5, 4), (C5, 4),
            (C5, 2), (E5, 2), (G5, 4), (G5, 4),
            (F5, 2), (E5, 2), (D5, 8),
            (D5, 2), (E5, 2), (F5, 4), (F5, 4),
            (E5, 2), (D5, 2), (E5, 4), (C5, 4),
            (C5, 2), (E5, 2), (D5, 4), (G4, 4),
            (B4, 2), (D5, 2), (C5, 8),
        ],
    },
    Song {
        name: "liang zhi lao hu (Two Tigers)",
        bpm: 120,
        notes: &[
            (C5, 4), (D5, 4), (E5, 4), (C5, 4),
            (C5, 4), (D5, 4), (E5, 4), (C5, 4),
            (E5, 4), (F5, 4), (G5, 8),
            (E5, 4), (F5, 4), (G5, 8),
            (G5, 2), (A5, 2), (G5, 2), (F5, 2), (E5, 4), (C5, 4),
            (G5, 2), (A5, 2), (G5, 2), (F5, 2), (E5, 4), (C5, 4),
            (C5, 4), (G4, 4), (C5, 8),
            (C5, 4), (G4, 4), (C5, 8),
        ],
    },
    Song {
        name: "Happy Birthday",
        bpm: 100,
        notes: &[
            (G4, 3), (G4, 1),
            (A4, 4), (G4, 4), (C5, 4),
            (B4, 8), (G4, 3), (G4, 1),
            (A4, 4), (G4, 4), (D5, 4),
            (C5, 8), (G4, 3), (G4, 1),
            (G5, 4), (E5, 4), (C5, 4),
            (B4, 4), (A4, 4), (F5, 3), (F5, 1),
            (E5, 4), (C5, 4), (D5, 4),
            (C5, 12),
        ],
    },
    Song {
        name: "It's a Small World",
        bpm: 174,
        notes: &[
            // It's a | world of laugh-ter, a | world of tears, it's a
            (E5, 2), (F5, 2),
            (G5, 4), (E6, 4), (C6, 4), (D6, 2), (C6, 2),
            (C6, 4), (B5, 4), (B5, 4), (D5, 2), (E5, 2),
            // world of hopes and a | world of fears, there's so
            (F5, 4), (D6, 4), (B5, 4), (C6, 2), (B5, 2),
            (A5, 4), (G5, 4), (G5, 4), (E5, 2), (F5, 2),
            // much that we share that it's | time we're a-ware it's a
            (G5, 4), (C6, 2), (D6, 2), (E6, 4), (D6, 2), (C6, 2),
            (A5, 4), (D6, 2), (E6, 2), (F6, 4), (E6, 2), (D6, 2),
            // small world af-ter | all. It's a（副歌）
            (G6, 4), (F6, 4), (E6, 4), (D6, 4),
            (C6, 12), (C5, 2), (C5, 2),
            // small world af-ter | all, it's a
            (E5, 4), (C5, 4), (D5, 4), (D5, 4),
            (D5, 12), (D5, 2), (D5, 2),
            (F5, 4), (D5, 4), (E5, 4), (E5, 4),
            (E5, 12), (E5, 2), (E5, 2),
            (G5, 4), (E5, 4), (F5, 4), (F5, 4),
            (F5, 12), (D5, 2), (C5, 2),
            // small, small | world.
            (G5, 8), (B5, 8),
            (C6, 16),
        ],
    },
    Song {
        name: "London Bridge Is Falling Down",
        bpm: 100,
        notes: &[
            (G5, 3), (A5, 1), (G5, 2), (F5, 2),
            (E5, 2), (F5, 2), (G5, 4),
            (D5, 2), (E5, 2), (F5, 4),
            (E5, 2), (F5, 2), (G5, 4),
            (G5, 3), (A5, 1), (G5, 2), (F5, 2),
            (E5, 2), (F5, 2), (G5, 4),
            (D5, 4), (G5, 4),
            (E5, 4), (C5, 4),
        ],
    },
];

w600::entry!(main);

fn sleep_ms(ms: u32) {
    thread::sleep(Duration::from_millis(ms as u64));
}

fn play(buzzer: &mut Pwm, song: &Song) {
    let sixteenth_ms = 60_000 / song.bpm / 4;
    for &(freq, sixteenths) in song.notes {
        let ms = sixteenths * sixteenth_ms;
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
}

fn main() {
    let p = w600::Peripherals::take().unwrap();
    let mut buzzer = Pwm::new(p.pwm3, p.pins.pb3, C5).unwrap();
    buzzer.set_duty(OFF_DUTY);

    loop {
        for song in SONGS {
            println!("playing: {}", song.name);
            play(&mut buzzer, song);
            sleep_ms(2000);
        }
    }
}
