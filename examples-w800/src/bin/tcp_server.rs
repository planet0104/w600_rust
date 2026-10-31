//! TCP 回显服务器：每个连接一个线程。连上后用 `nc <ip> 8088` 或网络调试助手（TCP 客户端）测试。
//!
//! 编译前修改下面的 SSID / PASSWORD。

#![no_std]
#![no_main]

use core::net::{Ipv4Addr, SocketAddrV4};
use core::time::Duration;

use embedded_io::{Read, Write};
use w600::net::{TcpListener, TcpStream};
use w600::{println, thread, wifi};

const SSID: &str = "REDMI K100 Pro Max";
const PASSWORD: &str = "jy123456";
const PORT: u16 = 8088;

w600::entry!(main);

fn main() {
    let info = loop {
        match wifi::connect(SSID, PASSWORD, Duration::from_secs(20)) {
            Ok(info) => break info,
            Err(e) => println!("wifi: {e}, retrying"),
        }
    };

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT)).unwrap();
    println!("echo server on {}:{PORT}", info.ip);
    loop {
        match listener.accept() {
            Ok((stream, peer)) => {
                println!("client {peer}");
                if thread::Builder::new().stack_size(4096).spawn(move || echo(stream)).is_err() {
                    println!("out of memory, dropping {peer}");
                }
            }
            Err(e) => println!("accept: {e}"),
        }
    }
}

fn echo(mut stream: TcpStream) {
    let mut buf = [0u8; 256];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 || stream.write_all(&buf[..n]).is_err() {
            break;
        }
    }
}
