// SPDX-License-Identifier: MIT
// Author: Uri Shaked

use std::ffi::c_void;

use wokwi_chip_ll::{i2cInit, I2CConfig};

use crate::gpio::GPIOPin;

pub struct I2CDeviceConfig {
    pub address: u32,
    pub scl: GPIOPin,
    pub sda: GPIOPin,

    pub connect_callback: Option<Box<dyn FnMut(u32, bool) -> bool + 'static>>,
    pub read_callback: Option<Box<dyn FnMut() -> u8 + 'static>>,
    pub write_callback: Option<Box<dyn FnMut(u8) -> bool + 'static>>,
    pub disconnect_callback: Option<Box<dyn FnMut() + 'static>>,
}

// This is a global registry of all the I2C devices, so that we can keep the Rust callbacks during
// the chip's lifetime.
static mut I2C_CONFIG_REGISTRY: Vec<*const I2CDeviceConfig> = Vec::new();

extern "C" fn i2c_connect_trampoline(user_data: *mut c_void, address: u32, write: bool) -> bool {
    let i2c_device = unsafe { &mut *(user_data as *mut I2CDeviceConfig) };
    if let Some(callback) = &mut i2c_device.connect_callback {
        callback(address, write)
    } else {
        false
    }
}

extern "C" fn i2c_read_trampoline(user_data: *mut c_void) -> u8 {
    let i2c_device = unsafe { &mut *(user_data as *mut I2CDeviceConfig) };
    if i2c_device.read_callback.is_some() {
        i2c_device.read_callback.as_mut().unwrap()()
    } else {
        0
    }
}

extern "C" fn i2c_write_trampoline(user_data: *mut c_void, data: u8) {
    let i2c_device = unsafe { &mut *(user_data as *mut I2CDeviceConfig) };
    if i2c_device.write_callback.is_some() {
        i2c_device.write_callback.as_mut().unwrap()(data);
    }
}

extern "C" fn i2c_disconnect_trampoline(user_data: *mut c_void) {
    let i2c_device = unsafe { &mut *(user_data as *mut I2CDeviceConfig) };
    if i2c_device.disconnect_callback.is_some() {
        i2c_device.disconnect_callback.as_mut().unwrap()();
    }
}

/// Create a new I2C device.
///
/// Example:
///
/// ```rust
/// use wokwi_chips_api::gpio::{GPIOPin, PinMode};
/// use wokwi_chips_api::i2c::{I2CDeviceConfig, create};
///
/// let scl = GPIOPin::new("SCL", PinMode::Output);
/// let sda = GPIOPin::new("SDA", PinMode::Output);
/// create(I2CDeviceConfig {
///     address: 0x42,
///     scl,
///     sda,
///     connect_callback: Some(Box::new(|address, write| {
///         println!("I2C connect: address=0x{:02x}, write={}", address, write);
///         true
///     })),
///     read_callback: Some(Box::new(|| {
///         println!("I2C read");
///         0x42
///     })),
///     write_callback: Some(Box::new(|data| {
///         println!("I2C write: 0x{:02x}", data);
///         true // ACK, false for NACK
///     })),
///     disconnect_callback: Some(Box::new(|| {
///         println!("I2C disconnect");
///     })),
/// });
/// ```
///
pub fn create(config: I2CDeviceConfig) {
    let ll_config = I2CConfig {
        user_data: &config as *const _ as *const c_void,
        address: config.address,
        scl: config.scl.get_id(),
        sda: config.sda.get_id(),
        connect: i2c_connect_trampoline as *const c_void,
        read: i2c_read_trampoline as *const c_void,
        write: i2c_write_trampoline as *const c_void,
        disconnect: i2c_disconnect_trampoline as *const c_void,
    };
    unsafe {
        i2cInit(&ll_config);
    }
    unsafe {
        I2C_CONFIG_REGISTRY.push(&config);
    }
}
