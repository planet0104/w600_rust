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

const SSID: &str = "REDMI K100 Pro Max";
const PASSWORD: &str = "jy123456";
/// (主机, 读超时秒数)
const TESTS: [(&str, u64); 2] = [("www.nmc.cn", 15), ("www.qq.com", 10)];

/// 与调试版 SDK 中 ip4.c 的 struct wm_dbg_pkt 一致。
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Pkt {
    time: u32,
    seq: u32,
    ack: u32,
    len: u16,
    wnd: u16,
    ipid: u16,
    frag: u16,
    rport: u16,
    lport: u16,
    dir: u8,
    flags: u8,
    err: i8,
    pad: u8,
}

unsafe extern "C" {
    fn wm_dbg_take(out: *mut Pkt, max: u32, lost: *mut u32) -> u32;
}

/// 取出并打印（print 为 false 时只清空）lwIP 记录的端口 443 的 TCP 包，序号换算成相对初始序号的值。
fn dump_packets(print: bool) {
    let mut pkts = [Pkt::default(); 160];
    let mut lost = 0;
    let n = unsafe { wm_dbg_take(pkts.as_mut_ptr(), pkts.len() as u32, &mut lost) } as usize;
    if !print {
        return;
    }
    if lost > 0 {
        println!("  ({lost} packet records lost)");
    }
    let pkts = &pkts[..n];
    let syn = |dir: u8| pkts.iter().find(|p| p.dir == dir && p.flags & 0x02 != 0).map_or(0, |p| p.seq);
    let (tx_isn, rx_isn) = (syn(b'T'), syn(b'R'));
    let t0 = pkts.first().map_or(0, |p| p.time);
    println!("  packets (T = sent, R = received, C = dropped by bad checksum):");
    for p in pkts {
        let (seq, ack) = if p.dir == b'T' {
            (p.seq.wrapping_sub(tx_isn), p.ack.wrapping_sub(rx_isn))
        } else {
            (p.seq.wrapping_sub(rx_isn), p.ack.wrapping_sub(tx_isn))
        };
        let mut flags = [b'.'; 5];
        for (i, (bit, c)) in [(0x02, b'S'), (0x10, b'A'), (0x08, b'P'), (0x01, b'F'), (0x04, b'R')].into_iter().enumerate() {
            if p.flags & bit != 0 {
                flags[i] = c;
            }
        }
        let flags = core::str::from_utf8(&flags).unwrap_or("?");
        if p.rport == 0 {
            println!(
                "  {:>6} {} IP fragment id={:04x} total_len={} offset={} more={}",
                p.time - t0,
                p.dir as char,
                p.ipid,
                p.len,
                (p.frag & 0x1fff) * 8,
                p.frag & 0x2000 != 0
            );
        } else {
            println!(
                "  {:>6} {} {flags} seq={seq} ack={ack} len={} wnd={} id={:04x} df={} port={} err={}",
                p.time - t0,
                p.dir as char,
                p.len,
                p.wnd,
                p.ipid,
                p.frag & 0x4000 != 0,
                p.lport,
                p.err
            );
        }
    }
}

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
    for (i, (host, timeout)) in TESTS.iter().enumerate() {
        println!("\n=== {host}, read timeout {timeout} s");
        dump_packets(false);
        let tcp = match TcpStream::connect_host(host, 443) {
            Ok(s) => s,
            Err(e) => {
                println!("tcp connect failed: {e}");
                continue;
            }
        };
        let mut tcp = tcp;
        tcp.set_read_timeout(Some(Duration::from_secs(*timeout)));
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
        dump_packets(true);
    }
    println!("\ndone");
}

extern crate alloc;
