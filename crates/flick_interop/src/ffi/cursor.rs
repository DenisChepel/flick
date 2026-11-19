//! The cursor position (WP-WINDOW; README section 4.3).

use crate::error::Result;
use crate::ffi::types::FlickPoint;
use crate::{panic, util};

/// The cursor position right now, in the space `flick_screens_*` reports screens in: physical
/// virtual-desktop pixels on Windows (`GetCursorPos`), CoreGraphics global points with a top-left
/// origin on macOS (the location of a fresh `CGEvent`, floored). Needs no window and no primary
/// screen. `out_point` is written only on `FLICK_OK`.
///
/// Status: `FLICK_OK`; `FLICK_S_NONE` when the OS will not say (`GetCursorPos` failed, as on a
/// secure desktop, or the window server made no event); `FLICK_E_INVALID_ARG` for a null
/// `out_point`; `FLICK_E_UNSUPPORTED` on a platform without a cursor backend.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_cursor_position(out_point: *mut FlickPoint) -> i32 {
    panic::guard("flick_cursor_position", || cursor_position(out_point))
}

unsafe fn cursor_position(out_point: *mut FlickPoint) -> Result {
    let out_point = util::out(out_point, "out_point")?;
    #[cfg(windows)]
    let point = flick_desktop::cursor_position().map(|p| (p.x, p.y));
    #[cfg(target_os = "macos")]
    let point = flick_desktop::cursor_location().map(|p| (p.x.floor() as i32, p.y.floor() as i32));
    #[cfg(any(windows, target_os = "macos"))]
    {
        match point {
            Some((x, y)) => {
                *out_point = FlickPoint {
                    x,
                    y,
                };
                Ok(crate::ffi::status::FLICK_OK)
            }
            None => Ok(crate::ffi::status::FLICK_S_NONE),
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = out_point;
        Err(crate::error::Error::unsupported("there is no cursor backend on this platform"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::status::{FlickInteropStatus, FLICK_OK, FLICK_S_NONE};

    #[test]
    fn a_null_out_is_invalid_arg() {
        assert_eq!(
            unsafe { flick_cursor_position(std::ptr::null_mut()) },
            FlickInteropStatus::InvalidArg as i32
        );
    }

    #[test]
    fn the_position_is_written_only_on_ok() {
        let mut point = FlickPoint {
            x: i32::MIN,
            y: i32::MIN,
        };
        let status = unsafe { flick_cursor_position(&mut point) };
        if !cfg!(any(windows, target_os = "macos")) {
            assert_eq!(status, FlickInteropStatus::Unsupported as i32);
            return;
        }
        if status == FLICK_S_NONE {
            assert_eq!(
                point,
                FlickPoint {
                    x: i32::MIN,
                    y: i32::MIN
                }
            );
            return;
        }
        assert_eq!(status, FLICK_OK);
        assert_ne!(
            point,
            FlickPoint {
                x: i32::MIN,
                y: i32::MIN
            }
        );
    }
}
