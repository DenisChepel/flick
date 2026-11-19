//! The IPC child's stdout guard (WP-IPC; README section 4.4), for the C# helpers
//! (Flick.VideoRender). The guard itself is `flick_ipc::guard`, the same code every Rust helper
//! runs through `flick_ipc::open`.

use std::io;

use crate::error::{Error, Result};
use crate::ffi::status::FLICK_OK;
use crate::{panic, util};

/// Claims the process stdout for the protocol (`flick_ipc::guard::claim`): a private,
/// non-inheritable copy of it goes to `out_handle` (a HANDLE on Windows, an fd elsewhere; ownership
/// passes to the caller, README C2), the original stdout's inherit flag is cleared (Windows), and
/// fd 1 is redirected to stderr: `STD_OUTPUT_HANDLE` and the UCRT's fd 1 on Windows, `dup2(2, 1)`
/// elsewhere. `out_redirected` is 1 when that redirect took and 0 when it failed (already logged to
/// stderr; stdout then still reaches the pipe), which is not an error. Both outputs are written
/// only on `FLICK_OK`.
///
/// Once per process: a second call (from this export or a Rust helper's `flick_ipc::open`) changes
/// nothing and answers `FLICK_E_STATE`.
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` for a null output; `FLICK_E_STATE` for a second call;
/// `FLICK_E_OS` when the duplicate failed (no usable stdout), with the OS error as the code.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_ipc_claim_stdout(out_handle: *mut isize, out_redirected: *mut u8) -> i32 {
    panic::guard("flick_ipc_claim_stdout", || claim_stdout(out_handle, out_redirected))
}

unsafe fn claim_stdout(out_handle: *mut isize, out_redirected: *mut u8) -> Result {
    let out_handle = util::out(out_handle, "out_handle")?;
    let out_redirected = util::out(out_redirected, "out_redirected")?;
    let claimed = flick_ipc::guard::claim().map_err(|e| match e.kind() {
        io::ErrorKind::AlreadyExists => Error::state(format!("flick_ipc_claim_stdout: {e}")),
        _ => Error::os(
            i64::from(e.raw_os_error().unwrap_or(0)),
            format!("flick_ipc_claim_stdout could not duplicate stdout: {e}"),
        ),
    })?;
    *out_handle = into_raw(claimed.file);
    *out_redirected = util::to_flag(claimed.redirected);
    Ok(FLICK_OK)
}

/// The private handle as the integer the caller takes ownership of.
fn into_raw(file: std::fs::File) -> isize {
    #[cfg(windows)]
    {
        use std::os::windows::io::IntoRawHandle;
        file.into_raw_handle() as isize
    }
    #[cfg(unix)]
    {
        use std::os::fd::IntoRawFd;
        file.into_raw_fd() as isize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::status::FlickInteropStatus;

    // The success path and the second-call refusal steal the test process's stdout, so they run in
    // a respawned child: tests/ipc_process.rs.

    #[test]
    fn null_outputs_are_invalid_arg_and_claim_nothing() {
        let mut handle = 0isize;
        let mut redirected = 0u8;
        assert_eq!(
            unsafe { flick_ipc_claim_stdout(std::ptr::null_mut(), &mut redirected) },
            FlickInteropStatus::InvalidArg as i32
        );
        assert_eq!(
            unsafe { flick_ipc_claim_stdout(&mut handle, std::ptr::null_mut()) },
            FlickInteropStatus::InvalidArg as i32
        );
        assert_eq!((handle, redirected), (0, 0));
    }
}
