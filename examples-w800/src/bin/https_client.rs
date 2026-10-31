//! 连接 Wi-Fi，然后通过 HTTPS（TLS 1.3）向服务器发送 GET 请求，打印响应的前 1KB 和总字节数。
//!
//! 编译前修改下面的 SSID / PASSWORD；HOST 只填主机名，路径和查询参数填在 PATH 中。
//!
//! 默认不校验服务器证书（只加密，无法防止中间人攻击）。需要校验时，把服务器证书链的根证书
//! 以 DER 格式保存到本文件旁边（例如在浏览器中查看网站证书，导出根证书为 DER 编码的 .cer 文件），
//! 再把 `CA` 改为 `Some(include_bytes!("root.cer"))`。

#![no_std]
#![no_main]

use core::time::Duration;

use embedded_io::{Read, Write};
use w600::net::TcpStream;
use w600::time::Instant;
use w600::tls::{Buffers, TlsStream};
use w600::{print, println, wifi};

const SSID: &str = "your-ssid";
const PASSWORD: &str = "your-password";
// 网址 https://www.nmc.cn/rest/weather?stationid=2011 拆成主机名和路径两部分。
const HOST: &str = "www.nmc.cn";
const PATH: &str = "/rest/weather?stationid=2011";
const CA: Option<&[u8]> = None;

// TLS 握手中的椭圆曲线和 RSA 运算需要较大的栈。
w600::entry!(main, stack_size = 24 * 1024);

fn main() {
    let info = loop {
        println!("connecting to {SSID} ...");
        match wifi::connect(SSID, PASSWORD, Duration::from_secs(30)) {
            Ok(info) => break info,
            Err(e) => println!("wifi: {e}, retrying"),
        }
    };
    println!("ip {} gateway {} dns {}", info.ip, info.gateway, info.dns);

    println!("connecting to {HOST}:443 ...");
    let mut tcp = match TcpStream::connect_host(HOST, 443) {
        Ok(s) => s,
        Err(e) => {
            println!("tcp connect failed: {e}");
            return;
        }
    };
    // 服务器没有响应时报错，而不是一直等待。
    tcp.set_read_timeout(Some(Duration::from_secs(10)));
    println!("tcp connected to {:?}, tls handshake ...", tcp.peer_addr());

    let mut buffers = Buffers::new();
    let start = Instant::now();
    let mut stream = match TlsStream::new(tcp, HOST, CA, &mut buffers) {
        Ok(s) => s,
        Err(e) => {
            println!("tls handshake failed after {} ms: {e}", start.elapsed().as_millis());
            return;
        }
    };
    println!(
        "tls connected in {} ms, free heap = {} bytes",
        start.elapsed().as_millis(),
        w600::system::free_heap()
    );

    let request = [
        "GET ", PATH, " HTTP/1.1\r\nHost: ", HOST, "\r\nUser-Agent: w600-rust\r\nConnection: close\r\n\r\n",
    ];
    let sent = request.iter().try_for_each(|part| stream.write_all(part.as_bytes())).and_then(|_| stream.flush());
    if let Err(e) = sent {
        println!("send failed: {e:?}");
        return;
    }

    // 只打印前 1KB（状态行和响应头），之后只统计字节数。
    let mut buf = [0u8; 512];
    let mut total = 0;
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if total < 1024 {
                    print!("{}", core::str::from_utf8(&buf[..n.min(1024 - total)]).unwrap_or("<binary>"));
                }
                total += n;
            }
            // 服务器发送 close_notify 或直接断开连接时结束。
            Err(_) => break,
        }
    }
    println!("\n\nreceived {total} bytes");
}
