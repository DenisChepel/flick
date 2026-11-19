//! Pure logic shared by Flick's Rust binaries (the capture overlay
//! `flick_capture`, the scrolling-capture driver `flick_scroll_driver`, the AI
//! inference binary `flick_ai` and the recorder stack) and by Flick.Ui and
//! Flick.VideoRender through `flick_interop`.
//!
//! The bar for living here is that **two processes must agree**, so a change
//! made in one place and not the other would be a bug:
//!
//! - [`geometry`] — the virtual-desktop coordinate space every capture
//!   coordinate, CLI flag and Win32 call is expressed in.
//! - [`session`] — the `session.json` contract, which `Flick.Ui` reads
//!   (`SessionInfo`) and both binaries write.
//! - [`ocr`] — the recognition request/response contract the overlay and
//!   `flick_ai ocr` speak across their process boundary.
//! - [`color`], [`share_region`], [`encoding`]: pure logic that a Rust
//!   helper and Flick.Ui (through flick_interop) must compute identically.
//! - [`contract`]: typed constant tables both sides read; flick_codegen
//!   renders them into C# (`cargo codegen`).
//!
//! Anything that only one binary needs stays in that binary. What the binaries
//! share but that reaches the OS or Sentry has a crate of its own: `flick_desktop`
//! and `flick_telemetry`.

pub mod color;
pub mod contract;
pub mod encoding;
pub mod geometry;
pub mod ocr;
pub mod session;
pub mod share_region;
