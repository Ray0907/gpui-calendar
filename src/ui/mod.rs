//! The application shell: window, sidebar, header and the Appearance screen.
//! `model` holds the state and is GPUI-free; the other files are views.

mod a11y;
mod appearance;
mod controls;
mod header;
pub mod model;
mod shell;
mod sidebar;

pub use shell::{Clock, MIN_WINDOW, RAIL_BELOW, Shell, local_clock, window_options};

#[cfg(test)]
mod tests;
