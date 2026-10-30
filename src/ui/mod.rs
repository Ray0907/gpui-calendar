//! The application shell: window, sidebar, header and the Appearance screen.
//! `model` holds the state and is GPUI-free; the other files are views.

mod a11y;
mod appearance;
mod controls;
mod header;
mod minipick;
pub mod model;
mod month;
mod popover;
mod shell;
mod sidebar;
mod week;
mod year;

pub use week::{DATE_CIRCLE, HEAD_H, LANE_H, lane_top};
pub use sidebar::{BANNER_EYEBROW, BANNER_META_LINE_OF, BANNER_TITLE, line_for};
pub use shell::{Clock, SHELL_MARGIN, MIN_WINDOW, RAIL_BELOW, Shell, local_clock, window_options};

#[cfg(test)]
mod tests;
