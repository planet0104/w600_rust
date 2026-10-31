//! TLS 1.3 客户端（基于 [`embedded-tls`](https://crates.io/crates/embedded-tls)），用于访问 HTTPS 等加密服务。
//! [`TlsStream`] 实现 `embedded-io` 的 `Read` / `Write`。
//!
//! ```no_run
//! use embedded_io::{Read, Write};
//! let mut buffers = w600::tls::Buffers::new();
//! let mut s = w600::tls::TlsStream::connect("www.example.com", 443, None, &mut buffers)?;
//! s.write_all(b"GET / HTTP/1.1\r\nHost: www.example.com\r\nConnection: close\r\n\r\n")?;
//! s.flush()?;
//! # Ok::<(), w600::tls::Error>(())
//! ```
//!
//! 限制：
//!
//! - 只支持 TLS 1.3（密码套件 TLS_AES_128_GCM_SHA256，密钥交换 P-256），只支持 TLS 1.2 的服务器无法连接；
//! - **不传 CA 证书时不校验服务器证书**：通信是加密的，但无法确认对方身份，可能遭受中间人攻击；
//! - 传入 CA 证书（DER 格式）时校验服务器发来的证书链能否连到该 CA，并校验域名；
//!   没有实时时钟，不检查证书有效期；
//! - 校验支持的签名算法：W800 为 ECDSA P-256 与 RSA；W600 受 Flash 容量限制只支持 ECDSA P-256。
//!   证书链中任何一级使用其他算法（如 ECDSA P-384）都会校验失败，不校验时不受此限制；
//! - 每个连接约占 30KB 堆内存；握手需要较大的线程栈：不校验证书时 24KB 足够，校验证书时需要约 48KB，
//!   栈不够时芯片会直接卡死而不报错；
//! - 随机数：W800 使用硬件真随机数发生器；W600 没有，种子取自芯片温度 ADC 的噪声与 CPU 计时抖动，强度较弱。

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use embedded_io::{ErrorType, Read, Write};
use embedded_tls::blocking::{
    Aes128GcmSha256, Certificate, CryptoProvider, NoClock, TlsConfig, TlsConnection, TlsContext, TlsError,
    TlsVerifier,
};
use embedded_tls::pki::CertVerifier;
use rand_chacha::ChaCha20Rng;
use rand_core::{CryptoRngCore, SeedableRng};

use crate::net::{self, TcpStream};
use crate::sys;

/// 最大的 TLS 记录（16KB 明文 + 加密开销），读缓冲区必须能容纳。
const READ_BUFFER_SIZE: usize = 16640;
const WRITE_BUFFER_SIZE: usize = 4096;
/// 校验时保存服务器证书链的空间。
const CERT_CHAIN_SIZE: usize = 8192;

/// 一个 TLS 连接使用的记录缓冲区（约 20KB，位于堆上），可在连接之间复用。
pub struct Buffers {
    read: Vec<u8>,
    write: Vec<u8>,
}

impl Buffers {
    pub fn new() -> Self {
        Self { read: vec![0; READ_BUFFER_SIZE], write: vec![0; WRITE_BUFFER_SIZE] }
    }
}

impl Default for Buffers {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// 建立 TCP 连接或 DNS 解析失败。
    Net(net::Error),
    /// TLS 握手失败（服务器不支持 TLS 1.3、证书校验失败等）。
    Tls(TlsError),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Net(e) => write!(f, "{e}"),
            Error::Tls(e) => write!(f, "tls error {e:?}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<TlsError> for Error {
    fn from(e: TlsError) -> Self {
        Error::Tls(e)
    }
}

/// 用于密钥生成的 32 字节随机种子。
#[cfg(target_arch = "csky")]
fn seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    unsafe { sys::tls_crypto_trng(seed.as_mut_ptr(), seed.len() as u32) };
    seed
}

/// 用于密钥生成的 32 字节随机种子。W600 没有真随机数发生器：混合芯片温度 ADC 读数的低位噪声、
/// 每次采样耗时造成的 SysTick 计数抖动和 MAC 地址，再做 SHA-256。
#[cfg(target_arch = "arm")]
fn seed() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    // Cortex-M3 的 SysTick 当前值寄存器（SYST_CVR），按 CPU 时钟递减。
    const SYST_CVR: *const u32 = 0xE000_E018 as *const u32;
    let mut hasher = Sha256::new();
    hasher.update(crate::system::mac_address());
    for _ in 0..32 {
        let temperature = unsafe { sys::adc_get_interTemp() };
        let cycles = unsafe { SYST_CVR.read_volatile() };
        let ticks = unsafe { sys::tls_os_get_time() };
        hasher.update(temperature.to_le_bytes());
        hasher.update(cycles.to_le_bytes());
        hasher.update(ticks.to_le_bytes());
    }
    hasher.finalize().into()
}

type Verifier<'a> = CertVerifier<'a, Aes128GcmSha256, NoClock, CERT_CHAIN_SIZE>;

struct Provider<'a> {
    rng: ChaCha20Rng,
    /// 约 8KB，放在堆上以免占用握手时的线程栈。
    verifier: Option<Box<Verifier<'a>>>,
}

impl CryptoProvider for Provider<'_> {
    type CipherSuite = Aes128GcmSha256;
    /// 只在客户端证书认证时使用，这里不支持。
    type Signature = &'static [u8];

    fn rng(&mut self) -> impl CryptoRngCore {
        &mut self.rng
    }

    fn verifier(&mut self) -> Result<&mut impl TlsVerifier<Aes128GcmSha256>, TlsError> {
        self.verifier.as_deref_mut().ok_or(TlsError::Unimplemented)
    }
}

/// TLS 连接。写入的数据会先缓存，调用 `flush` 后才会发出。
pub struct TlsStream<'a> {
    conn: TlsConnection<'a, TcpStream, Aes128GcmSha256>,
}

impl<'a> TlsStream<'a> {
    /// 解析 `host` 并建立 TCP 连接，然后完成 TLS 握手（SNI 与证书域名均使用 `host`）。
    ///
    /// `ca` 为 DER 格式的根证书；为 `None` 时不校验服务器证书。
    pub fn connect(host: &str, port: u16, ca: Option<&[u8]>, buffers: &'a mut Buffers) -> Result<Self, Error> {
        let tcp = TcpStream::connect_host(host, port).map_err(Error::Net)?;
        Self::new(tcp, host, ca, buffers)
    }

    /// 在已建立的 TCP 连接上完成 TLS 握手，例如需要先设置读超时的情况。
    pub fn new(tcp: TcpStream, host: &str, ca: Option<&[u8]>, buffers: &'a mut Buffers) -> Result<Self, Error> {
        #[allow(unused_mut)]
        let mut config = TlsConfig::new().with_server_name(host);
        // W800 开启了 RSA 支持，默认就会提供 RSA 签名算法。W600 不校验证书时同样提供，
        // 以便连接使用 RSA 证书的服务器；校验时只能提供能验证的算法。
        #[cfg(target_arch = "arm")]
        if ca.is_none() {
            config = config.enable_rsa_signatures();
        }
        let Buffers { read, write } = buffers;
        let mut conn = TlsConnection::new(tcp, read, write);
        let provider = Provider {
            rng: ChaCha20Rng::from_seed(seed()),
            verifier: ca.map(|der| Box::new(Verifier::new(Certificate::X509(der)))),
        };
        conn.open(TlsContext::new(&config, provider))?;
        Ok(Self { conn })
    }

    /// 通知服务器关闭连接（发送 close_notify），然后关闭 TCP 连接。
    pub fn close(self) -> Result<(), Error> {
        self.conn.close().map(drop).map_err(|(_, e)| Error::Tls(e))
    }
}

impl ErrorType for TlsStream<'_> {
    type Error = TlsError;
}

impl Read for TlsStream<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, TlsError> {
        self.conn.read(buf)
    }
}

impl Write for TlsStream<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, TlsError> {
        self.conn.write(buf)
    }

    fn flush(&mut self) -> Result<(), TlsError> {
        self.conn.flush()
    }
}
