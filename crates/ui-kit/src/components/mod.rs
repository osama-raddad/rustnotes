//! The Vynatix component library, one module per component. Every
//! component wraps the shared foundations ([`crate::states`],
//! [`crate::theme`]) so the focus ring, the state overlays and the kit
//! flags behave the same everywhere.

pub mod badge;
pub mod banner;
pub mod brand_showcase;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod chip;
pub mod cover;
pub mod editorial_card;
pub mod focus_ring;
pub mod liquid_glass;
pub mod metric_card;
pub mod radio;
pub mod select;
pub mod snackbar;
pub mod tabs;
pub mod text_field;
pub mod themes_showcase;
pub mod toggle;
pub mod top_app_bar;

pub use badge::{count_text, Badge, BadgeLook, BadgeTone, IconButton};
pub use banner::{banner_look, Banner, BannerEvent, BannerLook, BannerResponse, BannerTone};
pub use brand_showcase::{copy_passes_voice_checks, BrandShowcase, PhotoGround};
pub use button::{emphasis_look, Button, ButtonEmphasis, ButtonResponse, ButtonSize};
pub use card::{Card, CardLook, CardPadding, CardVariant};
pub use checkbox::{checkbox_look, Checkbox, CheckboxLook, CheckboxResponse, CheckboxState};
pub use chip::{chip_look, Chip, ChipEvent, ChipLook, ChipResponse};
pub use cover::{band_layout, BandRole, Cover};
pub use editorial_card::{EditorialCard, ImagePlaceholder};
pub use focus_ring::FocusRing;
pub use liquid_glass::{
    glass_recipe, paint_glass, GlassRecipe, GlassThickness, GlassTint, LiquidGlass,
};
pub use metric_card::MetricCard;
pub use radio::{
    option, radio_look, step_selection, RadioGroup, RadioLook, RadioOption, RadioResponse,
};
pub use select::{select_look, Select, SelectInteraction, SelectResponse};
pub use snackbar::{
    duration_ms, snackbar_look, Snackbar, SnackbarEvent, SnackbarHost, SnackbarLook, SnackbarStatus,
};
pub use tabs::{clamp_index, tab_look, TabLook, Tabs, TabsResponse};
pub use text_field::{field_look, FieldLook, TextField, TextFieldResponse};
pub use themes_showcase::{ShowcaseRole, ThemesShowcase};
pub use toggle::{ease_vynatix, toggle_look, travel_fraction, Toggle, ToggleLook, ToggleResponse};
pub use top_app_bar::{
    bar_border, bar_height, bar_recipe, link_look, LinkLook, NavLink, TopAppBar,
};
