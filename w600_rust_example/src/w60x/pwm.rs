#![allow(dead_code)]
use super::w60x_api;

// PWM1: A00,B18,B05,A05,B19,B30
// PWM2: A01,B17,B04,A07,B13,B20
// PWM3: A02,B16,B03,A08,B21
// PWM4: A03,B15,B02,A09,B22,B06
// PWM5: A04,B14,B01,A10,B23,B08

pub const CHANNEL_PWM1: u8 = 0;
pub const CHANNEL_PWM2: u8 = 1;
pub const CHANNEL_PWM3: u8 = 2;
pub const CHANNEL_PWM4: u8 = 3;
pub const CHANNEL_PWM5: u8 = 4;

pub fn pwm1_config(io_name: w60x_api::tls_io_name) {
    unsafe {
        w60x_api::wm_pwm1_config(io_name);
    }
}

pub fn pwm2_config(io_name: w60x_api::tls_io_name) {
    unsafe {
        w60x_api::wm_pwm2_config(io_name);
    }
}

pub fn pwm3_config(io_name: w60x_api::tls_io_name) {
    unsafe {
        w60x_api::wm_pwm3_config(io_name);
    }
}

pub fn pwm4_config(io_name: w60x_api::tls_io_name) {
    unsafe {
        w60x_api::wm_pwm4_config(io_name);
    }
}

pub fn pwm5_config(io_name: w60x_api::tls_io_name) {
    unsafe {
        w60x_api::wm_pwm5_config(io_name);
    }
}

/**
 * @brief          This function is used to initial pwm
 *
 * @param[in]      channel    pwm channel, range from 0 to 4
 * @param[in]      freq       is a pointer to frequency, freq range from 1 to 156250
 * @param[in]      duty       is a pointer to duty radio, duty range from 0 to 255
 * @param[in]      pnum       period num,range from 0 to 255
 *
 * @retval         success
 */
pub fn pwm_init(channel: u8, freq: u32, duty: u8, pnum: u8) -> bool {
    unsafe { w60x_api::tls_pwm_init(channel, freq, duty, pnum) == w60x_api::WM_SUCCESS }
}

/**
 * @brief          This function is used to start pwm
 *
 * @param[in]      channel    pwm channel, range from 0 to 4
 *
 * @retval         success
 *
 * @note           None
 */
pub fn pwm_start(channel: u8) -> bool {
    unsafe { w60x_api::tls_pwm_start(channel) == w60x_api::WM_SUCCESS }
}

/// duty: 0~255
pub fn pwm_duty_set(channel: u8, duty: u8) {
    unsafe {
        w60x_api::tls_pwm_duty_set(channel, duty);
    }
}
