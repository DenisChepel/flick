//! The audio output device (WP-AUDIO; README section 4.6): the default output, pulling
//! interleaved float32 from a render callback. Windows answers `FLICK_E_UNSUPPORTED` this phase
//! (NAudio's WASAPI output serves it, R15).

use std::ffi::c_void;

use crate::error::Error;
use crate::ffi::status::FLICK_OK;
use crate::panic;
use crate::util;

#[cfg(target_os = "macos")]
use crate::platform::macos::audio_output as os;

/// The audio output handle.
pub struct FlickAudioOutput {
    _private: [u8; 0],
}

/// What `flick_audio_output_create` reports about the opened output (8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlickAudioOutputInfo {
    /// The device's IO buffer plus its own latency plus its safety offset, in frames at
    /// `sample_rate_hz`; 0 when the device publishes none of the three (keep the requested figure).
    pub latency_frames: u32,
    /// The rate `latency_frames` counts in: the device side of the output unit (the requested rate
    /// when the device format cannot be read).
    pub sample_rate_hz: u32,
}

/// The render callback: fill all `frames * channels` interleaved float32 samples (pad silence on
/// an underrun; the device plays whatever is in the buffer). Runs on the OS audio thread; must not
/// block, allocate or take a lock the control path holds, and must not call any export (README C3).
pub type FlickAudioRenderCallback = Option<unsafe extern "C" fn(user: *mut c_void, samples: *mut f32, frames: u32, channels: u32)>;

/// The most channels `flick_audio_output_create` accepts (it bounds the stream-format arithmetic).
pub const FLICK_AUDIO_OUTPUT_MAX_CHANNELS: u32 = 4096;

/// Opens the default output (float32 interleaved at `sample_rate_hz`, `channels` in 1..=4096) and
/// installs `render` (with `user`, opaque, handed back unchanged), initialised but not started:
/// the unit is instantiated, formatted, callback-installed and initialised, and `out_info` carries
/// its measured latency. Nothing is left open on failure. Binds a device (tens of milliseconds on a
/// cold HAL), so it belongs on a play path, not a paint path.
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` (zero rate, channels out of range, null callback or
/// out pointer); `FLICK_E_CAPABILITY` (no default output unit); `FLICK_E_OS` (an AudioToolbox call
/// failed, OSStatus in the last-error code); `FLICK_E_UNSUPPORTED` on Windows and off macOS.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_audio_output_create(
    sample_rate_hz: u32,
    channels: u32,
    render: FlickAudioRenderCallback,
    user: *mut c_void,
    out_output: *mut *mut FlickAudioOutput,
    out_info: *mut FlickAudioOutputInfo,
) -> i32 {
    panic::guard("flick_audio_output_create", || {
        let out_output = util::out(out_output, "out_output")?;
        let out_info = util::out(out_info, "out_info")?;
        if sample_rate_hz == 0 {
            return Err(Error::invalid_arg("sample_rate_hz was zero"));
        }
        if channels == 0 || channels > FLICK_AUDIO_OUTPUT_MAX_CHANNELS {
            return Err(Error::invalid_arg(format!(
                "channels {channels} is not in 1..={FLICK_AUDIO_OUTPUT_MAX_CHANNELS}"
            )));
        }
        let render = render.ok_or_else(|| Error::null("render"))?;
        #[cfg(target_os = "macos")]
        {
            let (output, info) = os::Output::create(sample_rate_hz, channels, render, user)?;
            *out_info = info;
            *out_output = Box::into_raw(output).cast();
            Ok(FLICK_OK)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (render, user, out_output, out_info);
            Err(Error::unsupported(
                "flick_audio_output_create is implemented on macOS only (NAudio serves Windows)",
            ))
        }
    })
}

/// Starts pulling (idempotent: `FLICK_OK` when already running). After it returns the callback may
/// already have run on the audio thread. A failed start leaves the handle valid and stopped, so a
/// later start retries.
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` (null handle); `FLICK_E_OS`; `FLICK_E_PANIC` (a
/// render callback trampoline panicked since the last call); `FLICK_E_UNSUPPORTED` off macOS.
///
/// Thread: any thread, never concurrently with another call on the same handle.
#[no_mangle]
pub unsafe extern "C" fn flick_audio_output_start(output: *mut FlickAudioOutput) -> i32 {
    panic::guard("flick_audio_output_start", || {
        #[cfg(target_os = "macos")]
        {
            os::Output::live(output)?.start()?;
            Ok(FLICK_OK)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = output;
            Err(Error::unsupported("flick_audio_output_start is implemented on macOS only"))
        }
    })
}

/// Stops pulling (idempotent). After it returns no further callback begins, but one already
/// running may still be in flight, so stopping never frees what the callback touches; only
/// `flick_audio_output_destroy` does. The unit stays initialised, so a later start resumes without
/// binding a device again.
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` (null handle); `FLICK_E_OS`; `FLICK_E_PANIC`;
/// `FLICK_E_UNSUPPORTED` off macOS.
///
/// Thread: any thread, never concurrently with another call on the same handle.
#[no_mangle]
pub unsafe extern "C" fn flick_audio_output_stop(output: *mut FlickAudioOutput) -> i32 {
    panic::guard("flick_audio_output_stop", || {
        #[cfg(target_os = "macos")]
        {
            os::Output::live(output)?.stop()?;
            Ok(FLICK_OK)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = output;
            Err(Error::unsupported("flick_audio_output_stop is implemented on macOS only"))
        }
    })
}

/// Stops, uninitialises and disposes the unit, then frees the handle; null is `FLICK_OK`. The order
/// is the whole contract with an in-flight callback: uninitialising tears the IO proc down and waits
/// out a callback already running, so when this returns the callback will never run again and the
/// caller may free whatever `user` points at. Teardown always runs; a teardown OSStatus and a
/// callback panic not yet reported are logged, not returned.
///
/// Status: `FLICK_OK`; `FLICK_E_UNSUPPORTED` off macOS (for a non-null handle).
///
/// Thread: any thread, never concurrently with another call on the same handle.
#[no_mangle]
pub unsafe extern "C" fn flick_audio_output_destroy(output: *mut FlickAudioOutput) -> i32 {
    panic::guard("flick_audio_output_destroy", || {
        if output.is_null() {
            return Ok(FLICK_OK);
        }
        #[cfg(target_os = "macos")]
        {
            os::Output::destroy(Box::from_raw(output.cast::<os::Output>()));
            Ok(FLICK_OK)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(Error::unsupported("flick_audio_output_destroy is implemented on macOS only"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn layouts_are_pinned() {
        assert_eq!(size_of::<FlickAudioOutputInfo>(), 8);
        assert_eq!(offset_of!(FlickAudioOutputInfo, latency_frames), 0);
        assert_eq!(offset_of!(FlickAudioOutputInfo, sample_rate_hz), 4);
    }
}
