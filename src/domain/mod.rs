//! Pure calendar domain: no GPUI import, no clock, no I/O.

pub mod dates;
pub mod drag;
pub mod event;
pub mod form;
pub mod layout;
pub mod month;
pub mod year;

#[cfg(test)]
mod tests;
