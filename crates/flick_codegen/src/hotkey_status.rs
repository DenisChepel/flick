//! The hotkey status output, `ui/Flick.Shared/Hotkeys/HotkeyErrorCodes.g.cs`: flick_hotkeys'
//! machine-readable status codes (`ERROR_CODES`, the one code table), which Flick.Ui keys its
//! user-visible hotkey texts on.

use std::fmt::Write as _;
use std::path::Path;

use flick_hotkeys::ERROR_CODES;

use crate::{cs, Output};

const CODES_SOURCE: &str = "the status code table in crates/flick_hotkeys/src/error.rs";

/// The status codes.
pub static OUTPUTS: &[Output] = &[Output {
    name: "hotkey-status",
    path: "ui/Flick.Shared/Hotkeys/HotkeyErrorCodes.g.cs",
    source: CODES_SOURCE,
    render: render_codes,
}];

fn render_codes(_root: &Path) -> anyhow::Result<String> {
    let mut out = cs::header("hotkey_status", CODES_SOURCE);
    out.push_str(
        r#"namespace Flick
{
    /// <summary>The machine-readable hotkey status codes (flick_hotkeys <c>Error::code()</c>, the
    /// supervisor's <c>stale_grant</c> and <c>invalid_gesture</c>), which the user-visible hotkey
    /// texts are keyed on.</summary>
    public static class HotkeyErrorCodes
    {
"#,
    );
    for (name, code) in ERROR_CODES {
        let _ = writeln!(out, "        public const string {name} = {};", cs::string_literal(code));
    }
    out.push_str("    }\n}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_shapes() {
        let codes = render_codes(Path::new(".")).unwrap();
        assert!(codes.contains("        public const string Permission = \"permission\";\n"));
        assert!(codes.contains("        public const string StaleGrant = \"stale_grant\";\n"));
        assert!(codes.contains("        public const string InvalidGesture = \"invalid_gesture\";\n"));
    }
}
