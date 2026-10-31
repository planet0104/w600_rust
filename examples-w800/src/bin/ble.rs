//! BLE 收发消息：以 "W800-Rust" 为名广播，手机用蓝牙调试工具（如 nRF Connect、LightBlue）连接后，
//! 在服务 0xFFE0 下的特征 0xFFE1 上：
//!
//! 1. 订阅通知（nRF Connect 中点击该特征的三箭头图标）；
//! 2. 写入数据，串口会打印收到的内容，并通过同一个特征回复 "ack: <收到的内容>"。

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
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
    println!("ble ready, name = {NAME}, service 0xFFE0, characteristic 0xFFE1");

    loop {
        match server.recv() {
            Event::Advertising => println!("advertising"),
            Event::Connected => println!("connected"),
            Event::Disconnected { reason } => println!("disconnected, reason = 0x{reason:02x}"),
            Event::Received(data) => {
                match core::str::from_utf8(&data) {
                    Ok(text) => println!("received {} bytes: {text}", data.len()),
                    Err(_) => println!("received {} bytes: {data:02x?}", data.len()),
                }

                let mut reply: Vec<u8> = b"ack: ".to_vec();
                reply.extend_from_slice(&data);
                // 超出当前 MTU 的部分截掉。
                reply.truncate(server.max_notify_len());
                match server.notify(&reply) {
                    Ok(()) => println!("replied {} bytes", reply.len()),
                    Err(e) => println!("reply failed: {e}"),
                }
            }
        }
    }
}
