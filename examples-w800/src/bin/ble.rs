//! BLE 收消息：以 "W800-Rust" 为名广播，手机用蓝牙调试工具（如 nRF Connect、LightBlue）连接后，
//! 向服务 0xFFF0 下的特征 0xFFF2 写入数据，串口会打印收到的内容。

#![no_std]
#![no_main]

use w600::ble::{Event, Server};
use w600::println;

const NAME: &str = "W800-Rust";

w600::entry!(main);

fn main() {
    let server = match Server::start(NAME) {
        Ok(s) => s,
        Err(e) => {
            println!("ble start failed: {e}");
            return;
        }
    };
    println!("ble ready, name = {NAME}, write to characteristic 0xFFF2");

    loop {
        match server.recv() {
            Event::Advertising => println!("advertising"),
            Event::Connected => println!("connected"),
            Event::Disconnected { reason } => println!("disconnected, reason = 0x{reason:02x}"),
            Event::Received(data) => match core::str::from_utf8(&data) {
                Ok(text) => println!("received {} bytes: {text}", data.len()),
                Err(_) => println!("received {} bytes: {data:02x?}", data.len()),
            },
        }
    }
}
