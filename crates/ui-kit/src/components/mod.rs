//! The Vynatix component library, one module per component. Every
//! component wraps the shared foundations ([`crate::states`],
//! [`crate::theme`]) so the focus ring, the state overlays and the kit
//! flags behave the same everywhere.

pub mod badge;
pub mod button;
pub mod card;
pub mod chip;
pub mod editorial_card;
pub mod focus_ring;
pub mod liquid_glass;
pub mod metric_card;

pub use badge::{count_text, Badge, BadgeLook, BadgeTone, IconButton};
pub use button::{emphasis_look, Button, ButtonEmphasis, ButtonResponse, ButtonSize};
pub use card::{Card, CardLook, CardPadding, CardVariant};
pub use chip::{chip_look, Chip, ChipEvent, ChipLook, ChipResponse};
pub use editorial_card::{EditorialCard, ImagePlaceholder};
pub use focus_ring::FocusRing;
pub use liquid_glass::{
    glass_recipe, paint_glass, GlassRecipe, GlassThickness, GlassTint, LiquidGlass,
};
pub use metric_card::MetricCard;
