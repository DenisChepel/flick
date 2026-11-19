//! Gesture parse, format and check (WP-KEYS; README section 4.9). The grammar, the display text
//! and the reserved tables are flick_hotkeys': C# keeps no gesture parser of its own.

use flick_hotkeys::reserved::{self, Os};
use flick_hotkeys::{Gesture, Key, Modifiers};

use crate::error::{Error, Result};
use crate::ffi::status::FLICK_OK;
use crate::ffi::types::{FlickGestureRefusal, FlickString};
use crate::{panic, util};

/// A gesture: a `Key` value and `Modifiers` bits (8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlickGesture {
    pub key: i32,
    pub modifiers: u32,
}

/// Which text `flick_gesture_format` writes.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlickGestureStyle {
    /// The settings form (`Control+Alt+Shift+Meta+<Key>`), OS-independent.
    Serialized = 0,
    /// Labels plus the OS word for Meta.
    Display = 1,
    /// The modifier part only (`Ctrl+Alt+`).
    ModifiersPrefix = 2,
}

crate::convert::ffi_enum!(FlickGestureStyle: i32 { Serialized, Display, ModifiersPrefix });

/// The OS whose conventions apply.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlickGestureOs {
    Current = 0,
    Windows = 1,
    MacOS = 2,
    Linux = 3,
}

crate::convert::ffi_enum!(FlickGestureOs: i32 { Current, Windows, MacOS, Linux });

impl FlickGestureOs {
    fn os(self) -> Os {
        match self {
            FlickGestureOs::Current => Os::current(),
            FlickGestureOs::Windows => Os::Windows,
            FlickGestureOs::MacOS => Os::MacOS,
            FlickGestureOs::Linux => Os::Linux,
        }
    }
}

/// The gesture raw C values name, or the refusal (and its text) for values that name none: a key
/// outside the key table, `Key::None`, or an unknown modifier bit. Registration and
/// `flick_gesture_check` both read raw gestures through it (`hotkeys_host` imports it), so the
/// two refuse alike.
pub(crate) fn gesture_from_raw(key: i32, modifiers: u32) -> std::result::Result<Gesture, (FlickGestureRefusal, String)> {
    let Some(key) = Key::from_i32(key) else {
        return Err((
            FlickGestureRefusal::InvalidKey,
            format!("{key} is not a key of the hotkey key table"),
        ));
    };
    if key == Key::None {
        return Err((FlickGestureRefusal::NoKey, flick_hotkeys::Error::NoKey.to_string()));
    }
    let Some(modifiers) = Modifiers::from_bits(modifiers) else {
        return Err((
            FlickGestureRefusal::InvalidModifiers,
            format!("the modifiers 0x{modifiers:X} carry an unknown bit"),
        ));
    };
    Ok(Gesture::new(key, modifiers))
}

/// The refusal a hotkey error is about a gesture, or None for one about the hook (it cannot start,
/// it stopped): the one table from flick_hotkeys' validation errors onto `FlickGestureRefusal`.
pub(crate) fn refusal_of(error: &flick_hotkeys::Error) -> Option<FlickGestureRefusal> {
    match error {
        flick_hotkeys::Error::NoKey => Some(FlickGestureRefusal::NoKey),
        flick_hotkeys::Error::ModifierKey => Some(FlickGestureRefusal::ModifierOnly),
        flick_hotkeys::Error::UnsupportedKey(_) => Some(FlickGestureRefusal::UnsupportedKey),
        flick_hotkeys::Error::Duplicate => Some(FlickGestureRefusal::Duplicate),
        flick_hotkeys::Error::Unsupported
        | flick_hotkeys::Error::PermissionDenied
        | flick_hotkeys::Error::HookFailed(_)
        | flick_hotkeys::Error::Stopped => None,
    }
}

/// What `flick_gesture_check` answers: the first refusal in registration's order, then `os`'s
/// reserved table.
fn check(key: i32, modifiers: u32, os: Os) -> FlickGestureRefusal {
    let gesture = match gesture_from_raw(key, modifiers) {
        Ok(gesture) => gesture,
        Err((refusal, _)) => return refusal,
    };
    if let Err(error) = gesture.validate_on(os) {
        // validate_on only ever refuses the gesture itself
        return refusal_of(&error).unwrap_or(FlickGestureRefusal::InvalidKey);
    }
    if reserved::is_system_reserved_on(gesture, os) {
        return FlickGestureRefusal::Reserved;
    }
    FlickGestureRefusal::None
}

/// The text `flick_gesture_format` writes; `FLICK_E_INVALID_ARG` for a key outside the key table
/// or an unknown modifier bit (`Key::None` is a key here: "None" serialized, empty displayed).
fn format(gesture: &FlickGesture, style: FlickGestureStyle, os: Os) -> Result<String> {
    let key =
        Key::from_i32(gesture.key).ok_or_else(|| Error::invalid_arg(format!("{} is not a key of the hotkey key table", gesture.key)))?;
    let modifiers = Modifiers::from_bits(gesture.modifiers)
        .ok_or_else(|| Error::invalid_arg(format!("the modifiers 0x{:X} carry an unknown bit", gesture.modifiers)))?;
    let gesture = Gesture::new(key, modifiers);
    Ok(match style {
        FlickGestureStyle::Serialized => gesture.to_string(),
        FlickGestureStyle::Display => gesture.display(os),
        FlickGestureStyle::ModifiersPrefix => modifiers.display_prefix(os),
    })
}

/// Parses UTF-8 gesture text with the one gesture grammar (`Gesture::from_str`): '+'-separated
/// parts, trimmed, empty parts dropped; the last is a key name, alias or label ("PrintScreen",
/// "Snapshot", "PrtScr"), every earlier one a modifier name or shorthand ("Control", "Ctrl",
/// "Cmd"), all ASCII case-insensitive; a trailing '+' after "Num" is the keypad plus. Both the
/// serialized and the display text parse back. "None" is a gesture with no key (`key` 0).
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` for text that is no gesture (the reason, such as
/// "unknown key 'X'", is the last error), text that is not UTF-8, or a null pointer.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_gesture_parse(text: *const u8, len: usize, out: *mut FlickGesture) -> i32 {
    panic::guard("flick_gesture_parse", || {
        let out = util::out(out, "out")?;
        let text = util::utf8(text, len, "text")?;
        let gesture: Gesture = text
            .parse()
            .map_err(|e: flick_hotkeys::ParseGestureError| Error::invalid_arg(e.to_string()))?;
        *out = FlickGesture {
            key: gesture.key as i32,
            modifiers: u32::from(gesture.modifiers.bits()),
        };
        Ok(FLICK_OK)
    })
}

/// Formats a gesture as UTF-8 text (mode 1 string, freed with `flick_string_free`):
/// - `Serialized`: the settings form, OS-independent ("Control+Shift+PrintScreen"; the modifiers
///   in the order Control, Alt, Shift, Meta, then the key's canonical name; "None" for no key).
/// - `Display`: what a person reads, with the key's short label and `os`'s word for Meta
///   ("Ctrl+Shift+PrtScr", "Cmd+1"); empty for no key.
/// - `ModifiersPrefix`: the modifiers alone as `Display` writes them, each followed by '+'
///   ("Ctrl+Alt+"); empty for none. The key is not written.
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` for a null pointer, an unknown style or OS, a key
/// outside the key table or an unknown modifier bit.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_gesture_format(gesture: *const FlickGesture, style: i32, os: i32, out_text: *mut FlickString) -> i32 {
    panic::guard("flick_gesture_format", || {
        let out_text = util::out(out_text, "out_text")?;
        let gesture = util::input(gesture, "gesture")?;
        let style = FlickGestureStyle::from_raw(style)?;
        let os = FlickGestureOs::from_raw(os)?.os();
        *out_text = FlickString::from_string(format(gesture, style, os)?);
        Ok(FLICK_OK)
    })
}

/// Checks a gesture the way registration does on `os`, then against `os`'s reserved table, and
/// writes the first `FlickGestureRefusal` (`None` when the gesture is acceptable): `InvalidKey`,
/// `NoKey`, `InvalidModifiers`, `ModifierOnly`, `UnsupportedKey` (no key code on `os`), then
/// `Reserved` (Alt+Tab on Windows, Cmd+Q on macOS, ...). Never `Duplicate`, which only
/// registration knows. A gesture's own values are answered as refusals, never as a failure.
///
/// Status: `FLICK_OK`; `FLICK_E_INVALID_ARG` for a null pointer or an unknown OS.
///
/// Thread: any thread.
#[no_mangle]
pub unsafe extern "C" fn flick_gesture_check(gesture: *const FlickGesture, os: i32, out_refusal: *mut i32) -> i32 {
    panic::guard("flick_gesture_check", || {
        let out_refusal = util::out(out_refusal, "out_refusal")?;
        let gesture = util::input(gesture, "gesture")?;
        let os = FlickGestureOs::from_raw(os)?.os();
        *out_refusal = check(gesture.key, gesture.modifiers, os) as i32;
        Ok(FLICK_OK)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn layouts_are_pinned() {
        assert_eq!(size_of::<FlickGesture>(), 8);
        assert_eq!(offset_of!(FlickGesture, key), 0);
        assert_eq!(offset_of!(FlickGesture, modifiers), 4);
    }
}
