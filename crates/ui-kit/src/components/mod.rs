//! The Vynatix component library, one module per component. Every
//! component wraps the shared foundations ([`crate::states`],
//! [`crate::theme`]) so the focus ring, the state overlays and the kit
//! flags behave the same everywhere.

pub mod card;
pub mod editorial_card;
pub mod focus_ring;
pub mod liquid_glass;
pub mod metric_card;

pub use card::{Card, CardLook, CardPadding, CardVariant};
pub use editorial_card::{EditorialCard, ImagePlaceholder};
pub use focus_ring::FocusRing;
pub use liquid_glass::{
    glass_recipe, paint_glass, GlassRecipe, GlassThickness, GlassTint, LiquidGlass,
};
pub use metric_card::MetricCard;
