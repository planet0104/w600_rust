pub mod gpio;
pub mod pwm;
pub mod task;
pub mod w60x_api;
pub mod websocket;
pub mod wifi;

#[macro_export]
macro_rules! wm_println {
    () => ({
        unsafe{ $crate::w60x::w60x_api::wm_printf("\n\0".as_ptr()) }
    });
    ($($arg:tt)*) => {{
        let mut res = format!($($arg)*);
        res.push_str("\0");
        unsafe{ $crate::w60x::w60x_api::wm_printf("%s\n\0".as_ptr(), res.as_ptr()) }
    }}
}

#[macro_export]
macro_rules! cstr {
    ($val:expr) => {
        concat!($val, "\0").as_ptr() as *mut u8
    };
}
