//! The owned string's free (README B6, mode 1).

use crate::error::{Error, Result};
use crate::ffi::status::FLICK_OK;
use crate::ffi::types::FlickString;
use crate::panic;

#[allow(dead_code)] // the area exports hand strings out through these
impl FlickString {
    /// The empty string: null and zero, which every consumer treats as "nothing to free".
    pub(crate) fn empty() -> FlickString {
        FlickString {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }

    /// Takes ownership of `text` as a boxed slice, so the free needs only the pointer and the
    /// length. An empty string never allocates.
    pub(crate) fn from_string(text: impl Into<String>) -> FlickString {
        let text = text.into();
        if text.is_empty() {
            return FlickString::empty();
        }
        let bytes = text.into_bytes().into_boxed_slice();
        let len = bytes.len();
        FlickString {
            ptr: Box::into_raw(bytes).cast::<u8>(),
            len,
        }
    }

    /// The bytes as text; the empty string for a null pointer.
    ///
    /// # Safety
    /// `self` was produced by [`FlickString::from_string`] and not yet released.
    pub(crate) unsafe fn as_str(&self) -> &str {
        if self.ptr.is_null() || self.len == 0 {
            return "";
        }
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr, self.len))
    }

    /// Releases the allocation and leaves the struct empty, so a second release is a no-op.
    ///
    /// # Safety
    /// `self` is empty or was produced by [`FlickString::from_string`].
    pub(crate) unsafe fn release(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(self.ptr, self.len)));
        }
        self.ptr = std::ptr::null_mut();
        self.len = 0;
    }
}

/// Frees a standalone owned string this library produced (mode 1) and zeroes the caller's copy,
/// so a second free is a no-op. Never pass a string owned by a containing allocation (mode 2) or
/// a borrowed one (mode 3).
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` for a null `s`.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_string_free(s: *mut FlickString) -> i32 {
    panic::guard("flick_string_free", || free(s))
}

unsafe fn free(s: *mut FlickString) -> Result {
    let s = s.as_mut().ok_or_else(|| Error::null("s"))?;
    s.release();
    Ok(FLICK_OK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::status::FlickInteropStatus;

    #[test]
    fn round_trips_utf8_and_frees_once() {
        let mut s = FlickString::from_string("Caméra intégrée 相机");
        assert_eq!(unsafe { s.as_str() }, "Caméra intégrée 相机");
        assert_eq!(unsafe { flick_string_free(&mut s) }, 0);
        assert!(s.ptr.is_null());
        assert_eq!(s.len, 0);
        assert_eq!(unsafe { flick_string_free(&mut s) }, 0);
    }

    #[test]
    fn empty_does_not_allocate_and_null_is_invalid_arg() {
        let s = FlickString::from_string("");
        assert!(s.ptr.is_null());
        assert_eq!(
            unsafe { flick_string_free(std::ptr::null_mut()) },
            FlickInteropStatus::InvalidArg as i32
        );
    }
}
