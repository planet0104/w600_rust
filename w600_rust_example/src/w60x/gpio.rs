#![allow(dead_code)]
use super::w60x_api;
pub use w60x_api::{
    tls_gpio_attr, tls_gpio_dir, tls_io_name, WM_IO_PA_00, WM_IO_PA_05, WM_IO_PB_04, WM_IO_PB_06,
    WM_IO_PB_16, WM_IO_PB_17, WM_IO_PB_18,
};

pub fn gpio_cfg(gpio_pin: tls_io_name, dir: tls_gpio_dir, attr: tls_gpio_attr) {
    unsafe {
        w60x_api::tls_gpio_cfg(gpio_pin, dir, attr);
    }
}

pub fn gpio_write(gpio_pin: tls_io_name, value: u8) {
    unsafe {
        w60x_api::tls_gpio_write(gpio_pin, value);
    }
}
