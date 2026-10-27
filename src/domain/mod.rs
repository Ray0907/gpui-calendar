//! Pure calendar domain: no GPUI import, no clock, no I/O.

pub mod dates;
pub mod drag;
pub mod event;
pub mod layout;

#[cfg(test)]
mod tests;
