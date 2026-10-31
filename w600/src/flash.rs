//! 内部 Flash 的用户数据区，可用来保存配置等少量数据。
//!
//! 1MB Flash 上 SDK 划出了 48KB 用户区。它的第一个扇区同时是 SDK 的“用户默认参数”区，
//! 这里跳过该扇区，对外提供剩余的 44KB。写入时 SDK 会自动完成扇区的读-擦-写，无需手动擦除。

use crate::sync::Mutex;
use crate::sys;

const SECTOR_SIZE: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 读写范围超出用户区。
    OutOfBounds,
    /// SDK 返回失败。
    Io(i32),
}

/// 用户区的访问句柄。各句柄之间通过内部锁串行化。
#[derive(Debug, Default)]
pub struct UserFlash {
    _private: (),
}

static LOCK: Mutex<()> = Mutex::new(());

impl UserFlash {
    pub fn new() -> Self {
        Self { _private: () }
    }

    fn base() -> u32 {
        unsafe { sys::USER_ADDR_START + SECTOR_SIZE }
    }

    /// 可用字节数。
    pub fn capacity(&self) -> u32 {
        unsafe { sys::USER_AREA_LEN - SECTOR_SIZE }
    }

    fn check(&self, offset: u32, len: usize) -> Result<u32, Error> {
        let end = (offset as u64) + len as u64;
        if end > self.capacity() as u64 {
            return Err(Error::OutOfBounds);
        }
        Ok(Self::base() + offset)
    }

    pub fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        let addr = self.check(offset, buf.len())?;
        let _guard = LOCK.lock();
        match unsafe { sys::tls_fls_read(addr, buf.as_mut_ptr(), buf.len() as u32) } {
            0 => Ok(()),
            e => Err(Error::Io(e)),
        }
    }

    pub fn write(&mut self, offset: u32, data: &[u8]) -> Result<(), Error> {
        let addr = self.check(offset, data.len())?;
        let _guard = LOCK.lock();
        // SDK 只读取 data，签名里的 *mut 不代表会修改。
        match unsafe { sys::tls_fls_write(addr, data.as_ptr().cast_mut(), data.len() as u32) } {
            0 => Ok(()),
            e => Err(Error::Io(e)),
        }
    }
}
