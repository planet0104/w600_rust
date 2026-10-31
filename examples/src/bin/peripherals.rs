//! PWM 呼吸灯、按键中断、UART1 回显、ADC 与 Flash 读写。

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

use embedded_io::{Read, Write};
use w600::adc::Adc;
use w600::flash::UserFlash;
use w600::gpio::{Input, Pull, Trigger};
use w600::pwm::Pwm;
use w600::sync::Semaphore;
use w600::uart::{Config, Uart};
use w600::{println, thread};

w600::entry!(main);

static BUTTON: Semaphore = Semaphore::new(0);
static PRESSES: AtomicU32 = AtomicU32::new(0);

fn main() {
    let p = w600::Peripherals::take().unwrap();

    // PB16 上的呼吸灯（低电平点亮，所以占空比取反）。
    let mut led = Pwm::new(p.pwm2, p.pins.pb16, 1_000).unwrap();
    thread::spawn(move || {
        loop {
            for duty in (0..=255u8).chain((0..=255u8).rev()) {
                led.set_duty(255 - duty);
                thread::sleep(Duration::from_millis(4));
            }
        }
    });

    // PA1 上的按键，按下时接地。
    BUTTON.init();
    let mut button = Input::new(p.pins.pa1, Pull::Up);
    button.on_interrupt(Trigger::FallingEdge, || {
        PRESSES.fetch_add(1, Ordering::Relaxed);
        BUTTON.release();
    });
    thread::spawn(move || {
        let _button = button;
        loop {
            BUTTON.acquire();
            println!("button pressed {} times", PRESSES.load(Ordering::Relaxed));
        }
    });

    // UART1（TX = PB12，RX = PB11）回显。
    let mut uart = Uart::new(p.uart1, p.pins.pb12, p.pins.pb11, Config::baudrate(115_200)).unwrap();
    thread::spawn(move || {
        let mut buf = [0u8; 64];
        loop {
            if let Ok(n) = uart.read(&mut buf) {
                let _ = uart.write_all(&buf[..n]);
            }
        }
    });

    // 启动计数保存在 Flash 用户区。
    let mut flash = UserFlash::new();
    let mut boot = [0u8; 4];
    flash.read(0, &mut boot).unwrap();
    let count = match u32::from_le_bytes(boot) {
        u32::MAX => 1,
        n => n + 1,
    };
    flash.write(0, &count.to_le_bytes()).unwrap();
    println!("boot count = {count}");

    let mut adc = Adc::new(p.adc);
    loop {
        let t = adc.temperature_millicelsius();
        println!("chip temperature = {}.{:03} C", t / 1000, (t % 1000).abs());
        thread::sleep(Duration::from_secs(5));
    }
}
