//! `ui/Flick.Interop/Generated/EnvVars.g.cs`: the environment variables several processes read,
//! from `flick_ipc::env::ALL`, so each is spelled once.

use std::fmt::Write as _;
use std::path::Path;

use crate::{cs, Output};

const SOURCE: &str = "the table in crates/flick_ipc/src/env.rs";

/// The shared environment variable names.
pub static OUTPUTS: &[Output] = &[Output {
    name: "env",
    path: "ui/Flick.Interop/Generated/EnvVars.g.cs",
    source: SOURCE,
    render,
}];

fn render(_root: &Path) -> anyhow::Result<String> {
    let mut out = cs::header("env", SOURCE);
    out.push_str(
        r#"namespace Flick.Interop
{
    /// <summary>The environment variables more than one Flick process reads.</summary>
    public static class EnvVars
    {
"#,
    );
    for (i, var) in flick_ipc::env::ALL.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&cs::summary(var.doc.trim(), "        "));
        let _ = writeln!(out, "        public const string {} = {};", var.name, cs::string_literal(var.value));
    }
    out.push_str("    }\n}\n");
    Ok(out)
}
