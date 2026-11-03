//! 连续旋转（360°）舵机测试：依次输出停转、两个方向的不同速度，以及从反向全速到正向全速的扫描，
//! 每一步都在串口打印脉宽，方便观察改装后的舵机是否正常、找出停止点和调速范围。
//!
//! 接线：舵机信号线接 PA1（脉冲由软件产生，任何 GPIO 都可以，改下面 `p.pins.pa1` 即可；不要用 TX0，否则看不到日志），
//! 红线接 5V，棕线接 GND，并与开发板共地。舵机不要用 3.3V 供电。
//!
//! 先在停转阶段确认舵机完全不动；不动就把 NEUTRAL_US 抄到 music_box，仍在慢慢转就微调 NEUTRAL_US。
//! 扫描阶段可以看出：脉宽偏离中点多少才开始转（死区），偏离多少之后不再变快（饱和），据此调整 music_box 的转速。

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

use w600::gpio::{Level, Output, Pin};
use w600::{println, sync, sys, thread};

/// 舵机停转时的脉宽（µs）。
const NEUTRAL_US: u32 = 1500;
/// 速度为 100% 时脉宽偏离 NEUTRAL_US 的量（µs）。
const FULL_SPEED_US: u32 = 500;

/// 舵机线程每 20ms 读取一次，输出这么宽的脉冲。
static PULSE_US: AtomicU32 = AtomicU32::new(NEUTRAL_US);

w600::entry!(main);

fn main() {
    let p = w600::Peripherals::take().unwrap();
    start_servo(p.pins.pa1);
    println!("servo360 test: neutral {NEUTRAL_US} us, full speed offset {FULL_SPEED_US} us");
    println!("positive speed = pulse above neutral (usually counterclockwise)");

    loop {
        step("stop: the servo should not move", 0, 3);

        println!("\n--- direction 1 (pulse > neutral) ---");
        for speed in [10, 25, 50, 75, 100] {
            step("direction 1", speed, 3);
        }
        step("stop", 0, 2);

        println!("\n--- direction 2 (pulse < neutral) ---");
        for speed in [10, 25, 50, 75, 100] {
            step("direction 2", -speed, 3);
        }
        step("stop", 0, 2);

        println!("\n--- sweep from -100% to +100% ---");
        for speed in (-100..=100).step_by(5) {
            step("sweep", speed, 1);
        }
        step("stop", 0, 3);
        println!("\n=== test finished, restarting ===\n");
    }
}

/// 以 `speed`（-100..=100，百分比）转动 `secs` 秒。
fn step(label: &str, speed: i32, secs: u64) {
    let pulse = (NEUTRAL_US as i32 + FULL_SPEED_US as i32 * speed / 100) as u32;
    PULSE_US.store(pulse, Ordering::Relaxed);
    println!("{label}: speed {speed:>4}%, pulse {pulse} us, hold {secs} s");
    thread::sleep(Duration::from_secs(secs));
}

/// 用软件产生舵机脉冲：每 20ms 在关中断的情况下输出一个 PULSE_US 宽的高电平。
fn start_servo(pin: Pin) {
    let mut clk = sys::tls_sys_clk::default();
    unsafe { sys::tls_sys_clk_get(&mut clk) };
    let cpu_mhz = clk.cpuclk;
    let mut out = Output::new(pin, Level::Low);
    thread::Builder::new()
        .name("servo")
        .priority(thread::MIN_PRIORITY_VALUE)
        .spawn(move || {
            loop {
                let cycles = PULSE_US.load(Ordering::Relaxed) * cpu_mhz;
                sync::critical(|| {
                    out.set_high();
                    busy_wait_cycles(cycles);
                    out.set_low();
                });
                thread::sleep(Duration::from_millis(18));
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
