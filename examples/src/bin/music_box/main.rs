//! TW-01（W600）音乐盒：按住哪个按钮开机，舵机（改装成连续旋转）就往哪边转，同时蜂鸣器循环播放儿歌；
//! 断电后再开机，从上次那首的下一首开始播放。接线、舵机改装和调试方法见同目录的 README.md。

#![no_std]
#![no_main]

mod songs;

use core::time::Duration;

use alloc::vec;
use songs::{REST, SONGS, Song};
use w600::flash::UserFlash;
use w600::gpio::{Input, Level, Output, Pin, Pull};
use w600::pwm::Pwm;
use w600::{println, sync, sys, thread};

/// 舵机停转时的脉宽（µs）。改装后舵机在两个按钮都没按时仍慢慢转动，就微调这个值。
const NEUTRAL_US: u32 = 1500;
/// 脉宽偏离 NEUTRAL_US 多少时视为全速（µs）。
const FULL_SPEED_US: u32 = 500;
/// 转速，全速的百分比。嫌快就调小。
const SPEED_PERCENT: u32 = 50;
/// 为 true 时不管按钮，一直输出 NEUTRAL_US 的停转脉冲，用于改装后校准停止点。
const CALIBRATE: bool = false;

/// 每个音结尾的静音时长，让相邻的同音能分开。
const GAP_MS: u32 = 20;
/// 发声时的占空比（50% 方波）。
const ON_DUTY: u8 = 128;
/// 静音时的占空比（常低）。低电平触发的蜂鸣器模块（模块上的三极管为 PNP）请改为 255（常高）。
const OFF_DUTY: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// 按住 PB11（RX1）的按钮开机：逆时针。
    Left,
    /// 按住 PB12（TX1）的按钮开机：顺时针。
    Right,
    /// 没有按钮（例如用下载器供电）或两个都按着：舵机不动。
    Stop,
}

w600::entry!(main);

extern crate alloc;

fn main() {
    let p = w600::Peripherals::take().unwrap();

    // 检测线外接了分压电阻（兼作下拉），W600 没有内部下拉，这里设为浮空输入。
    let left = Input::new(p.pins.pb11, Pull::None);
    let right = Input::new(p.pins.pb12, Pull::None);
    let direction = match (left.is_high(), right.is_high()) {
        (true, false) => Direction::Left,
        (false, true) => Direction::Right,
        _ => Direction::Stop,
    };

    let mut log = SongLog::open();
    let mut index = log.last.map_or(0, |last| (last as usize + 1) % SONGS.len());
    println!("music box: direction {direction:?}, start from song {index}");

    let mut buzzer = Pwm::new(p.pwm1, p.pins.pa1, songs::C5).unwrap();
    buzzer.set_duty(OFF_DUTY);

    // 舵机接在 TX0 上，启动舵机后串口日志就看不到了。
    if CALIBRATE {
        start_servo(p.pins.pa4, NEUTRAL_US);
    } else if direction != Direction::Stop {
        start_servo(p.pins.pa4, pulse_us(direction));
    }

    loop {
        log.save(index as u8);
        println!("playing: {}", SONGS[index].name);
        play(&mut buzzer, &SONGS[index]);
        sleep_ms(2000);
        index = (index + 1) % SONGS.len();
    }
}

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

fn pulse_us(direction: Direction) -> u32 {
    let offset = FULL_SPEED_US * SPEED_PERCENT / 100;
    match direction {
        Direction::Left => NEUTRAL_US + offset,
        Direction::Right => NEUTRAL_US - offset,
        Direction::Stop => NEUTRAL_US,
    }
}

/// 用软件产生舵机脉冲：每 20ms 在关中断的情况下输出一个 `pulse_us` 宽的高电平。
/// 硬件 PWM 在 50Hz 下一级约 78µs，连续旋转舵机的调速范围只有中点附近一两百微秒，档位太少。
fn start_servo(pin: Pin, pulse_us: u32) {
    let mut clk = sys::tls_sys_clk::default();
    unsafe { sys::tls_sys_clk_get(&mut clk) };
    let cycles = pulse_us * clk.cpuclk;
    let mut out = Output::new(pin, Level::Low);
    thread::Builder::new()
        .name("servo")
        .priority(thread::MIN_PRIORITY_VALUE)
        .spawn(move || {
            loop {
                sync::critical(|| {
                    out.set_high();
                    busy_wait_cycles(cycles);
                    out.set_low();
                });
                sleep_ms(18);
            }
        })
        .unwrap();
}

/// 用 SysTick 当前值寄存器计时的忙等。它按 CPU 时钟递减，关中断时照常计数，每个系统节拍（2ms）回绕一次。
fn busy_wait_cycles(cycles: u32) {
    const SYST_RVR: *const u32 = 0xE000_E014 as *const u32;
    const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
    let period = unsafe { SYST_RVR.read_volatile() } + 1;
    let mut last = unsafe { SYST_CVR.read_volatile() };
    let mut elapsed = 0;
    while elapsed < cycles {
        let now = unsafe { SYST_CVR.read_volatile() };
        elapsed += if now <= last { last - now } else { last + period - now };
        last = now;
    }
}

/// 播放记录，存放在 Flash 用户区的最后一个扇区：每条 2 字节（歌曲索引及其反码），依次往后追加，
/// 写满一个扇区才擦除一次。往全是 0xFF 的位置写入时 SDK 不会擦除扇区，所以每次换歌不会磨损 Flash。
struct SongLog {
    flash: UserFlash,
    base: u32,
    /// 下一条记录的偏移（相对 base）。
    next: u32,
    /// 最后一条有效记录。
    last: Option<u8>,
}

const SECTOR_SIZE: u32 = 4096;
const RECORD_SIZE: u32 = 2;

impl SongLog {
    fn open() -> Self {
        let mut flash = UserFlash::new();
        let base = flash.capacity() - SECTOR_SIZE;
        let mut next = SECTOR_SIZE;
        let mut last = None;
        let mut chunk = [0u8; 256];
        'scan: for offset in (0..SECTOR_SIZE).step_by(chunk.len()) {
            flash.read(base + offset, &mut chunk).unwrap();
            for (i, &[index, check]) in chunk.as_chunks::<{ RECORD_SIZE as usize }>().0.iter().enumerate() {
                match (index, check) {
                    (0xFF, 0xFF) => {
                        next = offset + i as u32 * RECORD_SIZE;
                        break 'scan;
                    }
                    (index, check) if check == !index => last = Some(index),
                    // 写到一半掉电的记录，跳过。
                    _ => {}
                }
            }
        }
        Self { flash, base, next, last }
    }

    fn save(&mut self, index: u8) {
        if self.next >= SECTOR_SIZE {
            // 扇区里有旧数据时，SDK 会先擦除整个扇区再写入，所以写满 0xFF 就是擦除。
            self.flash.write(self.base, &vec![0xFF; SECTOR_SIZE as usize]).unwrap();
            self.next = 0;
        }
        self.flash.write(self.base + self.next, &[index, !index]).unwrap();
        self.next += RECORD_SIZE;
        self.last = Some(index);
    }
}
