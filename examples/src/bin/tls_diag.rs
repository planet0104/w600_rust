//! 临时诊断程序：与 w600::tls 使用相同的 TLS 配置，打印握手期间每次 TCP 读写的字节数与时间。
//! 依次尝试几个网站，定位握手停在哪一步。

#![no_std]
#![no_main]

use core::net::SocketAddrV4;
use core::time::Duration;

use embedded_io::{ErrorType, Read, Write};
use embedded_tls::blocking::{Aes128GcmSha256, CryptoProvider, TlsConfig, TlsConnection, TlsContext};
use rand_chacha::ChaCha20Rng;
use rand_core::{CryptoRngCore, SeedableRng};
use w600::net::{self, TcpStream};
use w600::time::Instant;
use w600::{println, wifi};

const SSID: &str = "your-ssid";
const PASSWORD: &str = "your-password";
/// (主机, 是否关闭 Wi-Fi 省电)
const TESTS: [(&str, bool); 4] = [("www.nmc.cn", false), ("www.nmc.cn", false), ("www.nmc.cn", true), ("www.nmc.cn", true)];

w600::entry!(main, stack_size = 24 * 1024);

/// 打印每次读写的 TCP 连接。
struct Logged {
    inner: TcpStream,
    start: Instant,
}

impl ErrorType for Logged {
    type Error = net::Error;
}

impl Read for Logged {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, net::Error> {
        let t = self.start.elapsed().as_millis();
        let r = self.inner.read(buf);
        match &r {
            Ok(n) => println!("  [{t:>5} ms] read  want {:>5} -> got {n} {:02x?}", buf.len(), &buf[..(*n).min(6)]),
            Err(e) => println!("  [{t:>5} ms] read  want {:>5} -> {e} (after {} ms)", buf.len(), self.start.elapsed().as_millis()),
        }
        r
    }
}

impl Write for Logged {
    fn write(&mut self, buf: &[u8]) -> Result<usize, net::Error> {
        let t = self.start.elapsed().as_millis();
        let r = self.inner.write(buf);
        println!("  [{t:>5} ms] write {:>5} -> {:?} {:02x?}", buf.len(), r, &buf[..buf.len().min(6)]);
        r
    }

    fn flush(&mut self) -> Result<(), net::Error> {
        self.inner.flush()
    }
}

struct Provider {
    rng: ChaCha20Rng,
}

impl CryptoProvider for Provider {
    type CipherSuite = Aes128GcmSha256;
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }
}

fn main() {
    loop {
        match wifi::connect(SSID, PASSWORD, Duration::from_secs(30)) {
            Ok(info) => {
                println!("ip {}", info.ip);
                break;
            }
            Err(e) => println!("wifi: {e}, retrying"),
        }
    }

    let mut read_buf = alloc::vec![0u8; 16640];
    let mut write_buf = alloc::vec![0u8; 4096];
    // 等 SDK 的系统任务按参数打开省电模式之后再开始测试。
    w600::thread::sleep(Duration::from_secs(1));
    for (i, (host, disable_ps)) in TESTS.iter().enumerate() {
        if *disable_ps {
            unsafe { w600::sys::tls_wifi_set_psflag(false as _, false as _) };
        }
        println!("\n=== {host}, wifi power save flag = {}", unsafe { w600::sys::tls_wifi_get_psflag() });
        let tcp = match TcpStream::connect_host(host, 443) {
            Ok(s) => s,
            Err(e) => {
                println!("tcp connect failed: {e}");
                continue;
            }
        };
        let mut tcp = tcp;
        tcp.set_read_timeout(Some(Duration::from_secs(10)));
        let peer: Option<SocketAddrV4> = tcp.peer_addr().ok();
        println!("tcp connected to {peer:?}");

        let config = TlsConfig::new().with_server_name(host).enable_rsa_signatures();
        let start = Instant::now();
        let mut conn = TlsConnection::new(Logged { inner: tcp, start }, &mut read_buf, &mut write_buf);
        // 诊断用的固定种子，不安全。
        let provider = Provider { rng: ChaCha20Rng::seed_from_u64(i as u64 + 1) };
        match conn.open(TlsContext::new(&config, provider)) {
            Ok(()) => println!("handshake OK in {} ms", start.elapsed().as_millis()),
            Err(e) => println!("handshake FAILED after {} ms: {e:?}", start.elapsed().as_millis()),
        }
    }
    println!("\ndone");
}

extern crate alloc;
