//! 联盟微 W600 官方 SDK（WM_SDK_W60X_G3.04.00）的原始 FFI 绑定。
//!
//! 绑定由 `cargo xtask bindgen` 从 SDK 头文件自动生成，SDK 静态库和 newlib 预编译后随 crate 一起发布，
//! 使用者不需要安装 SDK、libclang 或 ARM 工具链。一般应使用上层的 `w600` crate，而不是直接调用这里的函数。
//!
//! 链接方式：使用 Rust 自带的 rust-lld，应用的 `.cargo/config.toml` 中传入 `-Tw600.x` 即可，详见仓库 README。

#![no_std]
#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::all
)]

include!("bindings.rs");

/// FreeRTOSConfig.h 中的 `configTICK_RATE_HZ`，即一个系统节拍为 2ms。
pub const configTICK_RATE_HZ: u32 = 500;

/// lwIP 的 `FIONBIO`，原定义 `_IOW('f', 126, unsigned long)` 是 bindgen 无法求值的宏。
pub const FIONBIO: core::ffi::c_long = 0x8004_667e_u32 as core::ffi::c_long;
