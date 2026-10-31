//! 连接 Wi-Fi，然后向 HTTP 服务器发送一个 GET 请求并打印响应。
//!
//! 编译前修改下面的 SSID / PASSWORD。

#![no_std]
#![no_main]

use core::time::Duration;

use embedded_io::{Read, Write};
use w600::net::TcpStream;
use w600::{println, wifi};

const SSID: &str = "REDMI K100 Pro Max";
const PASSWORD: &str = "jy123456";
const HOST: &str = "baidu.com";

w600::entry!(main, stack_size = 12 * 1024);

fn main() {
    let info = loop {
        println!("connecting to {SSID} ...");
        match wifi::connect(SSID, PASSWORD, Duration::from_secs(20)) {
            Ok(info) => break info,
            Err(e) => println!("wifi: {e}, retrying"),
        }
    };
    println!("ip {} gateway {} dns {}", info.ip, info.gateway, info.dns);

    let mut stream = match TcpStream::connect_host(HOST, 80) {
        Ok(s) => s,
        Err(e) => {
            println!("connect failed: {e}");
            return;
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(10)));
    let request = "GET / HTTP/1.0\r\nHost: example.com\r\nConnection: close\r\n\r\n";
    if let Err(e) = stream.write_all(request.as_bytes()) {
        println!("send failed: {e}");
        return;
    }

    let mut buf = [0u8; 512];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => w600::print!("{}", core::str::from_utf8(&buf[..n]).unwrap_or("<binary>")),
            Err(e) => {
                println!("\nread failed: {e}");
                break;
            }
        }
    }
    println!("\ndone");
}
