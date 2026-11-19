//! One export that produces a chosen outcome on demand, so the status, last-error, log, panic and
//! re-entry machinery can be tested from Rust and from C# with no OS, GPU or permission behind it.
//! It ships in the release library on purpose: a diagnostic that exists only in debug builds is
//! missing exactly when it is needed.

use crate::error::{self, Error, Result};
use crate::ffi::log::{self, FlickInteropLogLevel};
use crate::ffi::status::{FlickInteropStatus, FLICK_OK, FLICK_S_CLOSED, FLICK_S_NONE, FLICK_S_TIMEOUT};
use crate::{convert, panic, util};

/// What `flick_interop_selftest` does.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlickSelftestKind {
    /// `FLICK_OK`, writing 42 to `out_value`.
    Ok = 0,
    /// `FLICK_S_NONE`, writing nothing.
    None = 1,
    /// `FLICK_S_TIMEOUT`.
    Timeout = 2,
    /// `FLICK_S_CLOSED`.
    Closed = 3,
    /// `FLICK_E_UNSUPPORTED` with a message.
    Unsupported = 4,
    /// `FLICK_E_INVALID_ARG` with a message.
    InvalidArg = 5,
    /// `FLICK_E_OS` with last-error code -2147467259 (`0x80004005` sign-extended from i32).
    Os = 6,
    /// A panic, which the guard turns into `FLICK_E_PANIC` naming this export.
    Panic = 7,
    /// `FLICK_E_STATE` with a message.
    State = 8,
    /// `FLICK_E_PERMISSION` with a message.
    Permission = 9,
    /// `FLICK_E_CAPABILITY` with a message.
    Capability = 10,
    /// `FLICK_E_STATE` with a message longer than 1 KiB (the buffer-growth path).
    LongMessage = 11,
    /// One log line at each `FlickInteropLogLevel`, then `FLICK_OK`.
    Log = 12,
    /// A nested guard entry: records `FLICK_E_STATE` "outer", runs an inner guard that fails with
    /// `FLICK_E_INVALID_ARG`, and returns `FLICK_E_STATE`; the last error must still be "outer".
    Nested = 13,
    /// A panic in an OS-callback trampoline that has no handle, caught onto the process slot, then
    /// `FLICK_OK`: the next export of any kind reports it to the log sink as one error line.
    CallbackPanic = 14,
}

convert::ffi_enum!(FlickSelftestKind: i32 {
    Ok, None, Timeout, Closed, Unsupported, InvalidArg, Os, Panic, State, Permission, Capability, LongMessage, Log, Nested,
    CallbackPanic
});

/// The message every failing selftest kind leaves; `LongMessage` repeats it.
const MESSAGE: &str = "the selftest was asked for this outcome";

/// Produces the outcome `kind` (a `FlickSelftestKind`) names; see that enum. `out_value` may be
/// null. Tests and diagnostics only.
///
/// Status: the one `kind` asks for; `FLICK_E_INVALID_ARG` for an unknown `kind`.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_interop_selftest(kind: i32, out_value: *mut i64) -> i32 {
    panic::guard("flick_interop_selftest", || run(kind, out_value))
}

unsafe fn run(kind: i32, out_value: *mut i64) -> Result {
    match FlickSelftestKind::from_raw(kind)? {
        FlickSelftestKind::Ok => {
            util::write_optional(out_value, 42);
            Ok(FLICK_OK)
        }
        FlickSelftestKind::None => Ok(FLICK_S_NONE),
        FlickSelftestKind::Timeout => Ok(FLICK_S_TIMEOUT),
        FlickSelftestKind::Closed => Ok(FLICK_S_CLOSED),
        FlickSelftestKind::Unsupported => Err(Error::unsupported(MESSAGE)),
        FlickSelftestKind::InvalidArg => Err(Error::invalid_arg(MESSAGE)),
        FlickSelftestKind::Os => Err(Error::os(0x8000_4005u32 as i32 as i64, "SelftestOsCall failed (0x80004005)")),
        FlickSelftestKind::Panic => panic!("the selftest was asked to panic"),
        FlickSelftestKind::State => Err(Error::state(MESSAGE)),
        FlickSelftestKind::Permission => Err(Error::permission(MESSAGE)),
        FlickSelftestKind::Capability => Err(Error::capability(MESSAGE)),
        FlickSelftestKind::LongMessage => Err(Error::state(format!("{MESSAGE}. ").repeat(40))),
        FlickSelftestKind::Log => {
            log::emit(FlickInteropLogLevel::Error, "selftest: error");
            log::emit(FlickInteropLogLevel::Warn, "selftest: warn");
            log::emit(FlickInteropLogLevel::Info, "selftest: info");
            log::emit(FlickInteropLogLevel::Debug, "selftest: debug");
            Ok(FLICK_OK)
        }
        FlickSelftestKind::Nested => {
            let state = FlickInteropStatus::State as i32;
            error::set(state, 0, "outer");
            let inner = panic::guard("flick_interop_selftest", || Err(Error::invalid_arg("inner")));
            debug_assert_eq!(inner, FlickInteropStatus::InvalidArg as i32);
            Ok(state)
        }
        FlickSelftestKind::CallbackPanic => {
            panic::callback_guard(panic::CallbackPanic::process(), (), || {
                panic!("the selftest callback was asked to panic")
            });
            Ok(FLICK_OK)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Slot;

    #[test]
    fn every_kind_produces_its_status() {
        use FlickInteropStatus as S;
        let expected = [
            (0, S::Ok),
            (1, S::None),
            (2, S::Timeout),
            (3, S::Closed),
            (4, S::Unsupported),
            (5, S::InvalidArg),
            (6, S::Os),
            (7, S::Panic),
            (8, S::State),
            (9, S::Permission),
            (10, S::Capability),
            (11, S::State),
            (12, S::Ok),
            (13, S::State),
            // 14 (CallbackPanic) leaves a panic for the next export to log; tests/infra.rs has it
        ];
        for (kind, status) in expected {
            let mut value = 0i64;
            assert_eq!(unsafe { flick_interop_selftest(kind, &mut value) }, status as i32, "kind {kind}");
            assert_eq!(value, if kind == 0 { 42 } else { 0 }, "kind {kind}");
        }
        assert_eq!(unsafe { flick_interop_selftest(99, std::ptr::null_mut()) }, S::InvalidArg as i32);
        assert!(error::snapshot().message.contains("99"));
    }

    #[test]
    fn failures_leave_a_message_and_the_os_code() {
        unsafe { flick_interop_selftest(6, std::ptr::null_mut()) };
        assert_eq!(
            error::snapshot(),
            Slot {
                message: "SelftestOsCall failed (0x80004005)".into(),
                code: -2147467259,
            }
        );
        unsafe { flick_interop_selftest(11, std::ptr::null_mut()) };
        assert!(error::snapshot().message.len() > 1024);
        unsafe { flick_interop_selftest(7, std::ptr::null_mut()) };
        assert!(error::snapshot()
            .message
            .contains("flick_interop_selftest panicked"));
    }

    #[test]
    fn a_nested_entry_keeps_the_outer_error() {
        assert_eq!(
            unsafe { flick_interop_selftest(13, std::ptr::null_mut()) },
            FlickInteropStatus::State as i32
        );
        assert_eq!(error::snapshot().message, "outer");
    }
}
