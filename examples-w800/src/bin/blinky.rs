//! 让 PB0 / PB1 / PB2 上的三个 LED 轮流闪烁（低电平点亮）。适用于 TB-08 开发板和 HLK-W800-KIT
//! （后者需打开板上的 LED 拨码开关），其他开发板请按原理图修改引脚。

#![no_std]
#![no_main]

use core::time::Duration;

use w600::gpio::{Level, Output};
use w600::{println, thread};

w600::entry!(main);

fn main() {
    let p = w600::Peripherals::take().unwrap();
    let mut leds = [
        Output::new(p.pins.pb0, Level::High),
        Output::new(p.pins.pb1, Level::High),
        Output::new(p.pins.pb2, Level::High),
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
