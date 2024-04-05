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

#[derive(Debug)]
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

pub struct Pin {
    id: PinId,
}

type WatchCallback = Box<dyn FnMut(Pin, PinValue) + 'static>;

struct PinListener {
    pin_id: PinId,
    callback: WatchCallback,
}

// This is a global registry of all the pins that have a watch set on them, so that we can keep the
// Rust callbacks alive as long as the watch is active.
static mut CALLBACK_REGISTRY: Vec<PinListener> = Vec::new();

extern "C" fn pin_change_trampoline(_user_data: *mut c_void, pin_id: PinId, value: u32) {
    let callback = unsafe {
        CALLBACK_REGISTRY
            .iter_mut()
            .find(|listener| listener.pin_id == pin_id)
            .map(|listener| &mut listener.callback)
    };

    if callback.is_none() {
        return;
    }

    callback.unwrap()(
        Pin { id: pin_id },
        if value == 0 {
            PinValue::Low
        } else {
            PinValue::High
        },
    );
}

impl Pin {
    pub fn new(name: &str, mode: PinMode) -> Self {
        let c_name = CString::new(name).unwrap();
        let id = unsafe { pinInit(c_name.as_ptr(), mode as u32) };

        Self { id }
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

    pub fn set_mode(&mut self, mode: PinMode) {
        unsafe {
            pinMode(self.id, mode as u32);
        }
    }

    pub fn get_id(&self) -> PinId {
        self.id
    }

    pub fn watch<F>(&self, edge: WatchEdge, callback: F) -> bool
    where
        F: FnMut(Pin, PinValue) + 'static,
    {
        let watch_config = WatchConfig {
            user_data: self as *const _ as *const c_void,
            edge: edge as u32,
            pin_change: pin_change_trampoline as *const c_void,
        };

        unsafe {
            CALLBACK_REGISTRY.push(PinListener {
                pin_id: self.id,
                callback: Box::new(callback),
            });
        }

        unsafe { pinWatch(self.id, &watch_config) }
    }

    pub fn unwatch(&self) {
        unsafe {
            pinWatchStop(self.id);
        }
    }
}
