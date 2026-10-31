//! 联盟微 W800 官方 SDK（W80X_SDK_v1.00.10）的原始 FFI 绑定。
//!
//! 绑定由 `cargo xtask w800-bindgen` 从 SDK 头文件自动生成，SDK 自带的预编译静态库随 crate 一起发布，
//! 使用者不需要安装 SDK 或 libclang。一般应使用上层的 `w600` crate，而不是直接调用这里的函数。
//!
//! 链接需要平头哥的 `csky-elfabiv2-gcc` 工具链（rust-lld 不支持 C-SKY），应用的 `.cargo/config.toml`
//! 中传入 `-Tw800.x` 即可，详见仓库 README。

#![no_std]
#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    unsafe_op_in_unsafe_fn,
    clippy::all
)]

include!("bindings.rs");

// lwIP 2.1 中这两个是宏，对应的实际函数名不同；按 W600（lwIP 2.0）的名字导出，方便上层统一使用。
pub use self::close as closesocket;
pub use self::lwip_gethostbyname as gethostbyname;

/// FreeRTOSConfig.h 中的 `configTICK_RATE_HZ`，即一个系统节拍为 2ms。
pub const configTICK_RATE_HZ: u32 = 500;

/// lwIP 的 `FIONBIO`，原定义 `_IOW('f', 126, unsigned long)` 是 bindgen 无法求值的宏。
pub const FIONBIO: core::ffi::c_long = 0x8004_667e_u32 as core::ffi::c_long;
