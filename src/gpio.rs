// SPDX-License-Identifier: MIT
// Author: Uri Shaked

use std::ffi::{c_void, CString};

use wokwi_chip_ll::{
    pinInit, pinMode, pinRead, pinWatch, pinWatchStop, pinWrite, PinId, WatchConfig, ANALOG, BOTH,
    FALLING, HIGH, INPUT, INPUT_PULLDOWN, INPUT_PULLUP, LOW, OUTPUT, OUTPUT_HIGH, OUTPUT_LOW,
    RISING,
};

use std::boxed::Box;

#[derive(Copy, Clone)]
pub enum PinMode {
    Input = INPUT as isize,
    Output = OUTPUT as isize,
    InputPullup = INPUT_PULLUP as isize,
    InputPulldown = INPUT_PULLDOWN as isize,
    Analog = ANALOG as isize,
    OutputLow = OUTPUT_LOW as isize,
    OutputHigh = OUTPUT_HIGH as isize,
}

pub enum PinValue {
    Low = LOW as isize,
    High = HIGH as isize,
}

impl std::ops::Not for PinValue {
    type Output = PinValue;

    fn not(self) -> PinValue {
        match self {
            PinValue::Low => PinValue::High,
            PinValue::High => PinValue::Low,
        }
    }
}

pub enum WatchEdge {
    Rising = RISING as isize,
    Falling = FALLING as isize,
    Both = BOTH as isize,
}

pub struct GPIOPin {
    id: PinId,
    mode: PinMode,

    watch_callback: Option<Box<dyn FnMut(PinValue) + 'static>>,
}

// This is a global registry of all the pins that have a watch set on them, so that we can keep the
// Rust callbacks alive as long as the watch is active.
static mut PIN_REGISTRY: Vec<*mut GPIOPin> = Vec::new();

extern "C" fn pin_change_trampoline(user_data: *mut c_void, _pin_id: u32, value: u32) {
    let pin = unsafe { &mut *(user_data as *mut GPIOPin) };
    pin.watch_callback.as_mut().unwrap()(if value == 0 {
        PinValue::Low
    } else {
        PinValue::High
    });
}

impl GPIOPin {
    pub fn new(name: &str, mode: PinMode) -> Self {
        let c_name = CString::new(name).unwrap();
        let id = unsafe { pinInit(c_name.as_ptr(), mode as u32) };
        Self {
            id,
            mode,
            watch_callback: None,
        }
    }

    pub fn read(&self) -> PinValue {
        unsafe {
            if pinRead(self.id) == 0 {
                PinValue::Low
            } else {
                PinValue::High
            }
        }
    }

    pub fn write(&self, value: PinValue) {
        unsafe {
            pinWrite(self.id, value as u32);
        }
    }

    pub fn set_low(&self) {
        self.write(PinValue::Low);
    }

    pub fn set_high(&self) {
        self.write(PinValue::High);
    }

    pub fn get_mode(&self) -> PinMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: PinMode) {
        self.mode = mode;
        unsafe {
            pinMode(self.id, mode as u32);
        }
    }

    pub fn get_id(&self) -> PinId {
        self.id
    }

    pub fn watch<F>(&mut self, edge: WatchEdge, callback: F) -> bool
    where
        F: FnMut(PinValue) + 'static,
    {
        // if a callback already exists, return false
        if self.watch_callback.is_some() {
            return false;
        }

        self.watch_callback = Some(Box::new(callback));

        let watch_config = WatchConfig {
            user_data: self as *mut _ as *const c_void,
            edge: edge as u32,
            pin_change: pin_change_trampoline as *const c_void,
        };

        let result = unsafe { pinWatch(self.id, &watch_config) };

        if result {
            unsafe {
                PIN_REGISTRY.push(&mut *(self as *const _ as *mut GPIOPin));
            }
        }

        result
    }

    pub fn unwatch(&self) {
        if self.watch_callback.is_none() {
            return;
        }

        unsafe {
            pinWatchStop(self.id);
        }

        unsafe {
            PIN_REGISTRY.retain(|&pin| pin != (self as *const _ as *mut GPIOPin));
        }
    }
}
