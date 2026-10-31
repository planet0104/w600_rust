//! 让 TB-01 开发板上的 RGB 灯轮流闪烁（PB16 红、PB17 蓝、PB18 绿，低电平点亮）。

#![no_std]
#![no_main]

use core::time::Duration;

use w600::gpio::{Level, Output};
use w600::{println, thread};

w600::entry!(main);

fn main() {
    let p = w600::Peripherals::take().unwrap();
    let mut leds = [
        Output::new(p.pins.pb16, Level::High),
        Output::new(p.pins.pb17, Level::High),
        Output::new(p.pins.pb18, Level::High),
    ];

    println!("blinky start, free heap = {} bytes", w600::system::free_heap());
    loop {
        for led in &mut leds {
            led.set_low();
            thread::sleep(Duration::from_millis(300));
            led.set_high();
        }
    }
}
