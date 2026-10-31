//! TCP / UDP（基于 SDK 内置的 lwIP 2.0.3 BSD socket）和 DNS 解析。
//! `TcpStream` 实现 `embedded-io` 的 `Read` / `Write`。
//!
//! SDK 的 lwIP 配置限制**同时最多 8 个 socket**（包括监听 socket）。

use core::ffi::{c_int, c_long, c_void};
use core::mem;
use core::net::{Ipv4Addr, SocketAddrV4};
use core::ptr;
use core::time::Duration;

use alloc::ffi::CString;
use embedded_io::{ErrorKind, ErrorType, Read, Write};

use crate::sync::Mutex;
use crate::sys;

/// socket 错误，内含 lwIP 的 errno。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    errno: i32,
}

impl Error {
    const TIMED_OUT: Error = Error { errno: sys::ETIMEDOUT as i32 };
    const INVALID_INPUT: Error = Error { errno: sys::EINVAL as i32 };
    const NOT_FOUND: Error = Error { errno: sys::ENOENT as i32 };

    /// lwIP 的 errno 值（取值与 Linux 相同）。
    pub fn errno(&self) -> i32 {
        self.errno
    }

    /// 全局 errno。lwIP 只有一个全局变量，多线程时可能被覆盖，仅在拿不到 socket 自身错误时使用。
    fn global() -> Error {
        let errno = unsafe { (&raw const sys::errno).read_volatile() };
        Error { errno: if errno == 0 { sys::EIO as i32 } else { errno } }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "socket error {:?} (errno {})", embedded_io::Error::kind(self), self.errno)
    }
}

impl core::error::Error for Error {}

impl embedded_io::Error for Error {
    fn kind(&self) -> ErrorKind {
        match self.errno as u32 {
            sys::ECONNREFUSED => ErrorKind::ConnectionRefused,
            sys::ECONNRESET => ErrorKind::ConnectionReset,
            sys::ECONNABORTED => ErrorKind::ConnectionAborted,
            sys::ENOTCONN => ErrorKind::NotConnected,
            sys::EADDRINUSE => ErrorKind::AddrInUse,
            sys::EADDRNOTAVAIL => ErrorKind::AddrNotAvailable,
            sys::EPIPE => ErrorKind::BrokenPipe,
            sys::EINVAL => ErrorKind::InvalidInput,
            sys::ETIMEDOUT | sys::EAGAIN => ErrorKind::TimedOut,
            sys::EINTR => ErrorKind::Interrupted,
            sys::ENOMEM | sys::ENOBUFS | sys::ENFILE => ErrorKind::OutOfMemory,
            sys::ENOENT => ErrorKind::NotFound,
            sys::EOPNOTSUPP => ErrorKind::Unsupported,
            _ => ErrorKind::Other,
        }
    }
}

/// [`TcpStream::shutdown`] 关闭的方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    Read,
    Write,
    Both,
}

fn to_sockaddr(addr: SocketAddrV4) -> sys::sockaddr_in {
    sys::sockaddr_in {
        sin_len: mem::size_of::<sys::sockaddr_in>() as u8,
        sin_family: sys::AF_INET as u8,
        sin_port: addr.port().to_be(),
        sin_addr: sys::in_addr { s_addr: u32::from_ne_bytes(addr.ip().octets()) },
        sin_zero: [0; 8],
    }
}

fn from_sockaddr(sa: &sys::sockaddr_in) -> SocketAddrV4 {
    SocketAddrV4::new(Ipv4Addr::from(sa.sin_addr.s_addr.to_ne_bytes()), u16::from_be(sa.sin_port))
}

struct Socket(c_int);

impl Socket {
    fn new(ty: u32) -> Result<Socket, Error> {
        let fd = unsafe { sys::socket(sys::AF_INET as c_int, ty as c_int, 0) };
        if fd < 0 { Err(Error::global()) } else { Ok(Socket(fd)) }
    }

    /// 读取并清除这个 socket 自己记录的错误（SO_ERROR），避免多线程共享全局 errno 的问题。
    fn error(&self) -> Error {
        let mut err: c_int = 0;
        let mut len = mem::size_of::<c_int>() as sys::socklen_t;
        let ret = unsafe {
            sys::getsockopt(self.0, sys::SOL_SOCKET as c_int, sys::SO_ERROR as c_int, (&raw mut err).cast(), &mut len)
        };
        if ret == 0 && err != 0 { Error { errno: err } } else { Error::global() }
    }

    fn check(&self, ret: c_int) -> Result<c_int, Error> {
        if ret < 0 { Err(self.error()) } else { Ok(ret) }
    }

    fn set_option<T>(&self, level: u32, name: u32, value: &T) -> Result<(), Error> {
        let ret = unsafe {
            sys::setsockopt(
                self.0,
                level as c_int,
                name as c_int,
                (value as *const T).cast::<c_void>(),
                mem::size_of::<T>() as sys::socklen_t,
            )
        };
        self.check(ret).map(drop)
    }

    /// SDK 的 lwIP 没有开启 SO_RCVTIMEO，读超时用 select 实现。
    fn wait_readable(&self, timeout: Option<Duration>) -> Result<(), Error> {
        let Some(timeout) = timeout else { return Ok(()) };
        let mut set = sys::fd_set { fd_bits: [0; 1] };
        set.fd_bits[self.0 as usize / 8] |= 1 << (self.0 % 8);
        let mut tv = sys::timeval {
            tv_sec: timeout.as_secs().min(c_long::MAX as u64) as c_long,
            tv_usec: timeout.subsec_micros() as c_long,
        };
        let n = unsafe { sys::select(self.0 + 1, &mut set, ptr::null_mut(), ptr::null_mut(), &mut tv) };
        match n {
            0 => Err(Error::TIMED_OUT),
            n => self.check(n).map(drop),
        }
    }

    fn local_addr(&self) -> Result<SocketAddrV4, Error> {
        let mut sa = sys::sockaddr_in::default();
        let mut len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        self.check(unsafe { sys::getsockname(self.0, (&raw mut sa).cast(), &mut len) })?;
        Ok(from_sockaddr(&sa))
    }

    fn bind(&self, addr: SocketAddrV4) -> Result<(), Error> {
        let sa = to_sockaddr(addr);
        let len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        self.check(unsafe { sys::bind(self.0, (&raw const sa).cast(), len) }).map(drop)
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { sys::closesocket(self.0) };
    }
}

/// TCP 连接。
pub struct TcpStream {
    sock: Socket,
    read_timeout: Option<Duration>,
}

impl TcpStream {
    pub fn connect(addr: SocketAddrV4) -> Result<TcpStream, Error> {
        let sock = Socket::new(sys::SOCK_STREAM)?;
        let sa = to_sockaddr(addr);
        let len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        sock.check(unsafe { sys::connect(sock.0, (&raw const sa).cast(), len) })?;
        Ok(TcpStream { sock, read_timeout: None })
    }

    /// 解析主机名后连接，例如 `TcpStream::connect_host("example.com", 80)`。
    pub fn connect_host(host: &str, port: u16) -> Result<TcpStream, Error> {
        Self::connect(SocketAddrV4::new(resolve(host)?, port))
    }

    /// 读超时；`None`（默认）表示一直等待。超时后 `read` 返回 `ErrorKind::TimedOut`。
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout;
    }

    pub fn set_nodelay(&self, nodelay: bool) -> Result<(), Error> {
        self.sock.set_option(sys::IPPROTO_TCP, sys::TCP_NODELAY, &(nodelay as c_int))
    }

    pub fn set_keepalive(&self, keepalive: bool) -> Result<(), Error> {
        self.sock.set_option(sys::SOL_SOCKET, sys::SO_KEEPALIVE, &(keepalive as c_int))
    }

    pub fn local_addr(&self) -> Result<SocketAddrV4, Error> {
        self.sock.local_addr()
    }

    pub fn peer_addr(&self) -> Result<SocketAddrV4, Error> {
        let mut sa = sys::sockaddr_in::default();
        let mut len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        self.sock.check(unsafe { sys::getpeername(self.sock.0, (&raw mut sa).cast(), &mut len) })?;
        Ok(from_sockaddr(&sa))
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), Error> {
        let how = match how {
            Shutdown::Read => sys::SHUT_RD,
            Shutdown::Write => sys::SHUT_WR,
            Shutdown::Both => sys::SHUT_RDWR,
        };
        self.sock.check(unsafe { sys::shutdown(self.sock.0, how as c_int) }).map(drop)
    }
}

impl ErrorType for TcpStream {
    type Error = Error;
}

impl Read for TcpStream {
    /// 阻塞直到读到数据；返回 0 表示对端已关闭连接。
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.sock.wait_readable(self.read_timeout)?;
        let n = unsafe { sys::recv(self.sock.0, buf.as_mut_ptr().cast(), buf.len(), 0) };
        self.sock.check(n).map(|n| n as usize)
    }
}

impl Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        let n = unsafe { sys::send(self.sock.0, buf.as_ptr().cast(), buf.len(), 0) };
        self.sock.check(n).map(|n| n as usize)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// TCP 监听 socket。
pub struct TcpListener {
    sock: Socket,
}

impl TcpListener {
    /// 监听 `addr`，监听所有地址时用 `SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)`。
    pub fn bind(addr: SocketAddrV4) -> Result<TcpListener, Error> {
        let sock = Socket::new(sys::SOCK_STREAM)?;
        let _ = sock.set_option(sys::SOL_SOCKET, sys::SO_REUSEADDR, &(1 as c_int));
        sock.bind(addr)?;
        sock.check(unsafe { sys::listen(sock.0, 4) })?;
        Ok(TcpListener { sock })
    }

    /// 等待一个新连接。
    pub fn accept(&self) -> Result<(TcpStream, SocketAddrV4), Error> {
        let mut sa = sys::sockaddr_in::default();
        let mut len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        let fd = self.sock.check(unsafe { sys::accept(self.sock.0, (&raw mut sa).cast(), &mut len) })?;
        Ok((TcpStream { sock: Socket(fd), read_timeout: None }, from_sockaddr(&sa)))
    }

    pub fn local_addr(&self) -> Result<SocketAddrV4, Error> {
        self.sock.local_addr()
    }
}

/// UDP socket。
pub struct UdpSocket {
    sock: Socket,
    read_timeout: Option<Duration>,
}

impl UdpSocket {
    /// 绑定本地地址；只发送时可以绑定 `SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)`。
    pub fn bind(addr: SocketAddrV4) -> Result<UdpSocket, Error> {
        let sock = Socket::new(sys::SOCK_DGRAM)?;
        sock.bind(addr)?;
        Ok(UdpSocket { sock, read_timeout: None })
    }

    pub fn send_to(&self, buf: &[u8], addr: SocketAddrV4) -> Result<usize, Error> {
        let sa = to_sockaddr(addr);
        let len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        let n = unsafe { sys::sendto(self.sock.0, buf.as_ptr().cast(), buf.len(), 0, (&raw const sa).cast(), len) };
        self.sock.check(n).map(|n| n as usize)
    }

    /// 接收一个数据报，返回长度和来源地址；超出 `buf` 的部分会被丢弃。
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddrV4), Error> {
        self.sock.wait_readable(self.read_timeout)?;
        let mut sa = sys::sockaddr_in::default();
        let mut len = mem::size_of::<sys::sockaddr_in>() as sys::socklen_t;
        let n = unsafe {
            sys::recvfrom(self.sock.0, buf.as_mut_ptr().cast(), buf.len(), 0, (&raw mut sa).cast(), &mut len)
        };
        let n = self.sock.check(n)?;
        Ok((n as usize, from_sockaddr(&sa)))
    }

    /// 读超时；`None`（默认）表示一直等待。
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout;
    }

    pub fn set_broadcast(&self, broadcast: bool) -> Result<(), Error> {
        self.sock.set_option(sys::SOL_SOCKET, sys::SO_BROADCAST, &(broadcast as c_int))
    }

    pub fn local_addr(&self) -> Result<SocketAddrV4, Error> {
        self.sock.local_addr()
    }
}

/// lwIP 的 gethostbyname 使用静态缓冲区，不可重入。
static DNS_LOCK: Mutex<()> = Mutex::new(());

/// 把主机名解析为 IPv4 地址；传入点分十进制地址时直接返回。
pub fn resolve(host: &str) -> Result<Ipv4Addr, Error> {
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Ok(ip);
    }
    let name = CString::new(host).map_err(|_| Error::INVALID_INPUT)?;
    let _guard = DNS_LOCK.lock();
    let he = unsafe { sys::gethostbyname(name.as_ptr()) };
    if he.is_null() {
        return Err(Error::NOT_FOUND);
    }
    unsafe {
        let list = (*he).h_addr_list;
        if list.is_null() || (*list).is_null() {
            return Err(Error::NOT_FOUND);
        }
        let octets = (*list).cast::<[u8; 4]>().read_unaligned();
        Ok(Ipv4Addr::from(octets))
    }
}
