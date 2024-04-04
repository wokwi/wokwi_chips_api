// SPDX-License-Identifier: MIT
// Author: Uri Shaked

use std::ffi::CString;

use wokwi_chip_ll::debugPrint;

#[macro_export]
macro_rules! println {
  ($($arg:tt)*) => {
      {
          use core::fmt::Write;
          writeln!($crate::Printer, $($arg)*).ok();
      }
  };
}

#[macro_export]
macro_rules! print {
  ($($arg:tt)*) => {
      {
          use core::fmt::Write;
          write!($crate::Printer, $($arg)*).ok();
      }
  };
}

pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            debugPrint(CString::new(s).unwrap().into_raw());
        }
        core::fmt::Result::Ok(())
    }
}
