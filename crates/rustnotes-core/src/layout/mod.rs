//! Mobile-first layout layer over egui.
//!
//! - [`Metrics`]: spacing, bar height, touch target and type scale, in points.
//! - [`WindowClass`]: Material width/height size classes from the viewport.
//! - [`Palette`]: colour tokens with a light and a dark instance.
//! - [`Scaffold`]: safe-area frame with top bar, body, bottom bar and FAB.
//! - [`Nav`]: route stack with a shared-axis slide and interactive pop.
//! - [`bottom_nav`]: tab bar for the scaffold's bottom slot.
//!
//! Everything is expressed in points; the shells supply the OS pixels-per-point
//! so a 48-point target is the same physical size on every device.

mod bottom_nav;
mod metrics;
mod nav;
mod palette;
mod scaffold;
mod size_class;
mod widgets;

pub use bottom_nav::{bottom_nav, icon_notes, icon_settings, NavItem};
pub use metrics::{Metrics, TextScale};
pub use nav::Nav;
pub use palette::Palette;
pub use scaffold::{bar, BarLine, Scaffold, ScaffoldRects};
pub use size_class::{SizeClass, WindowClass};
pub use widgets::{add_button, back_button, bar_title};
