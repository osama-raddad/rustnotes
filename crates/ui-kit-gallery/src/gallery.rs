//! The Vynatix gallery in its final shape (issues 09, 10 and 11): a
//! Liquid Glass TopAppBar carrying the wordmark, the primary nav, the
//! search affordance, the theme menu and the one call to action; a
//! Tabs strip switching the gallery's panels — the Tabs widget's own
//! real demo — over a scrolling page; and, on the Feedback tab, the
//! Banner demos at the panel top and the Snackbar host's trigger
//! buttons, the snackbar itself floating in near the window's bottom.
//!
//! The panel grouping (the judgment issue 09 documented, extended by
//! issue 10): the Foundations tab holds the type specimen, the
//! interaction states, the foundation specimens (spacing, radii,
//! elevation), the icons, the logo and the colour-role swatches; the
//! Components tab holds the actions (buttons, chips, badges), the
//! surfaces (cards, metrics, editorial, glass), the text inputs and
//! the choice controls (checkboxes, radio groups, toggles); the
//! Feedback tab holds the banners and the snackbars; the Brand tab
//! holds the gallery's three brand-book pages. The bar's nav links
//! and the tab strip drive the same panel state — the link's
//! `aria-current`-style underline and the tab's indicator are the two
//! accents the specs give them. Themes and the reduced-motion and
//! reduced-transparency flags live in the bar's theme menu (a kit
//! button opening a kit popover: a radio group and two toggles), all
//! six themes reachable.
//!
//! The Brand tab and the cover (issue 10's judgment, documented): the
//! issue wants the Cover as the gallery's first screen, and the
//! gallery's chrome is the bar-plus-tabs of issue 09. The
//! reconciliation is a Cover panel as the FIRST tab, named Brand: the
//! gallery opens on the cover composition (eyebrow, display
//! headline, standfirst, the mark, the band), and the Themes and
//! Brand showcases follow it on the same panel. The cover is a panel
//! rather than an intro overlay so the final chrome stays put (the
//! bar and the tab strip never disappear), and it is the first tab
//! rather than folded under Foundations so the brand book opens the
//! gallery the way the source system's brand book opens on its cover.
//! The theme cards on that panel need every theme's faces, so
//! startup installs all six themes' fonts (the installs are additive
//! by design); the theme menu's per-switch install stays as it was.
//!
//! The polish pass (issue 11): every string the gallery paints is
//! routed through `ui_kit::gallery_copy` as a constant, where the
//! voice lint scans it (no em dashes, no stray ellipses, no
//! exclamation marks, no emoji, no forbidden vocabulary, sentence
//! case for headings); the specimen rows that can outgrow a narrow
//! window (the state tiles, the radii, the metrics, the editorial
//! trio, the glass panes) scroll horizontally instead of clipping
//! their tails, the way the web previews scroll the same rows.
//!
//! The phone pass: under the design system's 600pt compact
//! breakpoint the bar keeps the wordmark, the theme menu (on
//! its bare label, or its icon alone on the narrowest phones)
//! and the search affordance; the caption under the tab strip
//! steps aside; the button rows and the Foundations specimens
//! wrap (the specimens through `wrapping_row`, since egui's
//! `horizontal_wrapped` never wraps a plain group); the card grid and
//! the form grids stack. The page, the sideways rows and the tab strip
//! scroll under the finger through egui's own drag-to-scroll, from the
//! first touch (lib.rs tells the input story).
//!
//! This module is the app's whole UI, hosted by the `rustnotes` host
//! through `GalleryApp` (lib.rs): the host supplies the platform glue
//! (safe areas, keyboard, appearance, wgpu limits, insets) and calls
//! [`Gallery::chrome`] as the root UI each frame. Run it on desktop with
//! `cargo run -p ui-kit-gallery`, on the iOS simulator with
//! `just run-ios app=ui-kit-gallery`, on a connected iPhone with
//! `just ios-install app=ui-kit-gallery`, and on Android with
//! `just run-android app=ui-kit-gallery`.

use egui::scroll_area::{DragScroll, ScrollSource};
use egui::{self, Align2, Color32, CornerRadius, Rect, Sense, Vec2};

use ui_kit::components::badge::{Badge, IconButton};
use ui_kit::components::banner::{Banner, BannerEvent, BannerTone};
use ui_kit::components::button::{self, Button, ButtonEmphasis, ButtonSize};
use ui_kit::components::checkbox::{Checkbox, CheckboxState};
use ui_kit::components::chip::{Chip, ChipEvent};
use ui_kit::components::liquid_glass::{glass_recipe, paint_glass, GlassThickness, GlassTint};
use ui_kit::components::radio::{self, option, RadioGroup, RadioOption};
use ui_kit::components::select::{popup_frame, Select};
use ui_kit::components::snackbar::{Snackbar, SnackbarEvent, SnackbarHost};
use ui_kit::components::tabs::Tabs;
use ui_kit::components::text_field::{hint_row, label_row, TextField};
use ui_kit::components::toggle::{self, Toggle};
use ui_kit::components::top_app_bar::{
    NavLink, TopAppBar, ACTIONS_GAP, BAR_PAD_X, BAR_PAD_Y, CONTENT_HEIGHT, GROUP_GAP, LINK_GAP,
    LOGO_HEIGHT,
};
use ui_kit::components::{
    BrandShowcase, Card, Cover, EditorialCard, FocusRing, Grid, LiquidGlass, MetricCard,
    ThemesShowcase,
};
use ui_kit::gallery_copy as copy;
use ui_kit::icon::{icon, icon_active, IconSize};
use ui_kit::icons::{Icon, CATEGORY_GROUPS};
use ui_kit::logo::Logo;
use ui_kit::states::{self, UiKitFlags};
use ui_kit::theme::{egui_shadow, ThemeId};
use ui_kit::tokens::{breakpoints, radii, spacing, ThemeColors};
use ui_kit::typography::{install, specimen_job, TrackedLabel, TypeStyle};

/// The width under which the gallery switches to its phone layout: the
/// bar drops its nav links (the Tabs strip below already switches
/// panels) and its call to action, and two-column grids stack. It is
/// the design system's own compact breakpoint, the 600 of the
/// `breakpoint-tablet` token, where its mobile layout ends.
const NARROW_WIDTH: f32 = breakpoints::BREAKPOINT_TABLET;

/// The page scroll area's id salt. An unsalted `ScrollArea` takes its
/// id from its `Ui` alone, so two unsalted areas in one `Ui` share one
/// `scroll_area::State`. The kit's Tabs strip, laid out just above the
/// page in the same `Ui`, once had no salt of its own, and sharing the
/// page's state the strip (which scrolls sideways only) clamped the
/// vertical offset back to zero every frame: neither the wheel nor a
/// finger could move the page for longer than a frame. The strip
/// carries its own salt now; this one keeps the page's state its own
/// whatever else the panel holds. [`page_scroll_salt`] pairs it with
/// the active panel, so each panel has a page state of its own.
const PAGE_SCROLL: &str = "gallery-page";

/// The salt source of the page scroll area for the panel at `tab`.
/// The area's state, and with it the scroll offset, lives under the
/// id this source makes; one source for every panel would carry the
/// offset from the panel the reader left to the one they open.
/// `page` takes both the area's salt and the id it loads the state
/// by from this one value: `ScrollArea::id_salt` hashes its argument
/// into an `IdSalt`, so a lookup by any other spelling reads a fresh
/// default state.
fn page_scroll_salt(tab: usize) -> (&'static str, usize) {
    (PAGE_SCROLL, tab)
}

/// The page's side gutter. The bar, the tab strip and the page all share
/// it, so the chrome's edges line up with the text rather than sitting
/// at two different insets.
pub(crate) const PAGE_GUTTER: f32 = spacing::SPACE_4;

/// The content panel's inner margin: the gutter at each side, and the
/// 24px rhythm above the tab strip and below the page.
fn content_margin() -> egui::Margin {
    egui::Margin {
        left: PAGE_GUTTER as i8,
        right: PAGE_GUTTER as i8,
        top: spacing::SPACE_6 as i8,
        bottom: spacing::SPACE_6 as i8,
    }
}

/// The window height under which the gallery's chrome compacts, for the
/// phone in landscape (finding 4-A2): the tab description is dropped (as
/// it is on a phone in portrait) and the chrome's vertical gaps give way
/// to [`SHORT_GAP`]. The window's height is the root `Ui`'s, the system
/// bars' insets included.
///
/// Landscape phones are 360 to 448pt tall (the Pixel 448, iPhones 375 to
/// 440), under 500; a portrait phone is 568pt or more and a landscape
/// tablet 600pt or more, over it. As a check, with no insets at all the
/// full chrome (the bar and the tab strip included) is 245pt tall with
/// the description on two lines and 263pt on three, so it leaves the page
/// half of the window only from 490 to 526pt up; insets only raise that.
pub(crate) const SHORT_WINDOW_HEIGHT: f32 = 500.0;

/// Each of the chrome's vertical gaps under the bar in a short window
/// (see [`SHORT_WINDOW_HEIGHT`]): the bar's clearance below, the content
/// panel's margin above the tab strip, and the space between the strip
/// and the page. `space-1`, because the room is tight: `space-2` would
/// take 25pt (three gaps and the bar's 1pt separator room) where the
/// Pixel in landscape has 19pt for them once the page has half the
/// window; the budget is in `a_short_window_keeps_half_its_height_for_the_page`.
/// The bar's clearance above stays as it is, so the bar, and the
/// popovers that hang from it, stand where they stand in a window of any
/// height. The content panel's margin below the page goes, since `page`
/// ends its scroll with 24pt of space of its own.
const SHORT_GAP: f32 = spacing::SPACE_1;

/// A drag on a sideways row picks its axis once it has travelled this
/// far: a mostly vertical gesture scrolls the page, a mostly horizontal
/// one scrolls the row, never both.
const AXIS_LOCK_DISTANCE: f32 = 6.0;

/// The offset change a sideways row hands the page for its next frame,
/// keyed in egui's temp memory. The page takes it at the start of its
/// frame, so the hand-off lags one frame.
const PAGE_FORWARD: &str = "gallery-page-forward";

/// The content height the page laid out last frame, in egui's temp
/// memory under the page's own id (so one height per panel, as the
/// scroll state is). `page` compares it with this frame's to tell a page
/// that changed height from one the reader only scrolled.
const PAGE_HEIGHT: &str = "gallery-page-height";

/// How far two content heights, or an offset and the page's furthest
/// offset, may differ and still count as the same: half a point, the
/// most the layout's pixel snap moves an edge at one device pixel per
/// point (at 3x a pixel is a third of a point and the snap is finer).
/// `page` uses it for both comparisons.
const PAGE_PIXEL_SNAP: f32 = 0.5;

/// One drag gesture on a sideways row: the axis it picked (`true` for
/// vertical), `None` until its travel passes [`AXIS_LOCK_DISTANCE`].
#[derive(Clone, Copy, Default)]
struct DragAxis {
    vertical: Option<bool>,
}

/// The theme menu's popover width ceiling, not its width: `.width` caps
/// the area, which shrinks to its content, the longest toggle row
/// ("Reduced transparency" beside its 44px track) and the frame's
/// insets, about 226px.
const THEME_MENU_WIDTH: f32 = 248.0;

/// The theme menu's scroll area's id salt, for the window too short to
/// show the whole menu (see [`bar_popover_room`]).
const THEME_MENU_SCROLL: &str = "gallery-theme-menu";

/// The x the theme menu's labels start at, in the theme rows and the
/// flag rows alike: past the toggle's 44px track and its gap. The
/// toggle's label cannot start sooner, so the radio group is indented
/// to meet it ([`THEME_MENU_RADIO_INSET`]).
const THEME_MENU_LABEL_X: f32 = toggle::TRACK_W + toggle::LABEL_GAP;

/// How far the theme menu's radio group is indented so its labels,
/// which the kit sets a circle and a gap from the row's left, start
/// at [`THEME_MENU_LABEL_X`]. The group's legend rides along with it.
const THEME_MENU_RADIO_INSET: f32 = THEME_MENU_LABEL_X - (radio::CIRCLE_SIZE + radio::LABEL_GAP);

/// The search popover's outer width, frame included: `.width` caps the
/// area and the field fills it.
pub(crate) const SEARCH_POPOVER_WIDTH: f32 = 280.0;

/// The gap between the bar's bottom edge and the popovers that open
/// under it: `space-2`, the bar's own `ACTIONS_GAP`. It clears the
/// bar's 1px border and its shadow's edge; the select's 4px, under a
/// field, would leave a popover touching the floating bar.
pub(crate) const BAR_POPOVER_GAP: f32 = spacing::SPACE_2;

/// The inner margin of the bar's popovers: `space-4`, the theme menu's.
/// The kit's popover frame keeps 4px, which suits the select's list of
/// flush rows; a field or a stack of 44px touch rows wants room around
/// it.
pub(crate) const BAR_POPOVER_PAD: f32 = spacing::SPACE_4;

/// The frame both of the bar's popovers draw: the kit's popover chrome
/// with the bar popovers' one inner margin, so the search field and the
/// theme menu's rows stand the same distance from the edge.
fn bar_popover_frame(theme: ThemeId) -> egui::Frame {
    popup_frame(theme).inner_margin(BAR_POPOVER_PAD)
}

/// The popover for one of the bar's triggers, under the bar. One rule
/// places both of the bar's popovers (the theme menu and the search
/// field):
///
/// - it starts [`BAR_POPOVER_GAP`] under the bar's bottom edge, not
///   under the trigger, which ends the bar's padding short of that edge
///   and would leave the popover over the bar's card, its border and
///   its hairline;
/// - its right edge stands on the trigger's right edge; and
/// - where it is wider than the room between that edge and the bar's
///   left edge (e.g. at a 384px phone, the 280px search popover under a
///   search button that ends 233px from the window's left edge, 217px
///   from the bar's), it is held inside the bar instead: pushed right
///   until its left edge stands on the bar's. Otherwise it would run
///   off the window's left edge.
///
/// egui's own placement tries `BOTTOM_START`, then its flips, then
/// every menu alignment, and takes the first that fits the window. That
/// hung the popovers by their left edges on a wide window and centred
/// the search popover under its button on a phone, so the alignment is
/// fixed here, with no alternatives. `bar` is the bar's pane and
/// `width` the popover's width ceiling, which `.width` sets. The hold
/// measures the popover as it was last laid out, its area's rect, and
/// falls back to `width` until the area has been laid out once: the
/// search popover fills its width, but the theme menu's area shrinks
/// to its content (about 226pt against the 248pt ceiling), and the
/// ceiling would push its right edge past the trigger's. The caller sets
/// the open state and shows the content.
fn bar_popover<'a>(
    trigger: &egui::Response,
    bar: Rect,
    width: f32,
    theme: ThemeId,
) -> egui::Popup<'a> {
    // The popover's width as last laid out: the area's rect, keyed by
    // the popup's id. A popup never laid out has none, and its first
    // sizing pass is invisible, so the next frame anchors it again. A
    // reopened popup reads the rect of its last opening, which is
    // content-sized and so not stale.
    let laid_out = trigger
        .ctx
        .memory(|memory| memory.area_rect(egui::Popup::default_response_id(trigger)))
        .map_or(width, |rect| rect.width());
    // The trigger's right edge, or the right edge that puts the
    // popover's left edge on the bar's, where the former would leave
    // the popover's left edge short of the bar's.
    let right = trigger.rect.right().max(bar.left() + laid_out);
    egui::Popup::from_response(trigger)
        .anchor(egui::pos2(right, bar.bottom()))
        .align(egui::RectAlign::BOTTOM_END)
        .align_alternatives(&[])
        .gap(BAR_POPOVER_GAP)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(bar_popover_frame(theme))
        .width(width)
}

/// The height a popover's content may take under the bar `bar` for the
/// popover to stay below the bar and inside the window: from the
/// [`BAR_POPOVER_GAP`] under the bar's bottom edge to the bottom of the
/// window's content rect (the window less its safe-area insets), less
/// the frame's own margin and border above and below the content.
///
/// egui does not shrink a popup that is too tall for the room under the
/// bar. It keeps the popup's size and moves the popup up until it fits
/// the content rect (`Area` constrains to it), over the bar, and where
/// the popup is taller than the whole content rect it stands the popup
/// at the rect's top and clips the rest. So a popup that can outgrow the
/// room, as the theme menu does in a landscape phone, scrolls its
/// content inside this height instead. A popup that fits is left where
/// [`bar_popover`] put it.
fn bar_popover_room(ctx: &egui::Context, bar: Rect, theme: ThemeId) -> f32 {
    let under_bar = ctx.content_rect().bottom() - (bar.bottom() + BAR_POPOVER_GAP);
    (under_bar - bar_popover_frame(theme).total_margin().sum().y).max(0.0)
}

/// Apply `theme` with the gallery's own style on top: labels are not
/// selectable. A gallery is not a document, and a drag that starts on
/// selectable text selects it instead of scrolling the page.
/// `ThemeId::apply` replaces the visuals alone, so the setting would
/// survive a switch anyway; every theme change goes through here all
/// the same, so the two never drift apart.
pub(crate) fn apply_theme(theme: ThemeId, ctx: &egui::Context) {
    theme.apply(ctx);
    ctx.all_styles_mut(|style| style.interaction.selectable_labels = false);
}

impl Gallery {
    /// The gallery's starting state: the Brand tab, every demo
    /// populated.
    pub(crate) fn new() -> Self {
        Self {
            theme: ThemeId::Light,
            flags: UiKitFlags::default(),
            filters: [false; 4],
            saved_chip: true,
            form: ProjectForm::populated(),
            choices: ChoiceDemos::populated(),
            tab: 0,
            page_scroll_id: None,
            page_rect: None,
            banners: BannerDemos::populated(),
            snackbar: SnackbarHost::new(),
            search: String::new(),
            search_open: false,
            theme_open: false,
        }
    }
}

/// When a drag scrolls the gallery's scroll areas (the page, the
/// sideways specimen rows and the kit's Tabs strip): from the very
/// first touch on a phone, and on desktop only once a touch has been
/// seen. egui's default, [`DragScroll::OnTouch`], turns drag-to-scroll
/// on only once egui has seen a touch, and egui binds a press to its
/// drag target against the previous frame's widgets: on a fresh launch
/// the first finger meets areas that registered no drag target the
/// frame before, so the first swipe moves nothing and only the later
/// ones scroll. A phone's pointer is a finger, so there the areas drag
/// [`DragScroll::Always`]. Desktop keeps the default: on a mouse-only
/// desktop a drag never scrolls (the wheel and the scroll bars do);
/// once the screen has been touched, egui arms it for every pointer.
/// The headless tests take the phone path, so they pin the first
/// swipe.
fn drag_scroll() -> DragScroll {
    if cfg!(any(target_os = "android", target_os = "ios", test)) {
        DragScroll::Always
    } else {
        DragScroll::OnTouch
    }
}

/// A specimen row that can outgrow the page: laid out horizontally
/// inside a horizontal scroll, so a narrow window scrolls the row
/// instead of clipping its tail. The web previews scroll the same
/// rows; `id` keeps the scroll areas distinct. A finger drags the row
/// on [`drag_scroll`]'s terms.
///
/// The row's drag is horizontal, and egui's horizontal area claims the
/// whole gesture, so a vertical swipe that began on the row used to
/// scroll nothing. The row therefore owns the gesture's axis: the first
/// [`AXIS_LOCK_DISTANCE`] of travel decides it. A vertical drag hands
/// its movement to the page through [`PAGE_FORWARD`] and holds the row
/// still; a horizontal drag scrolls the row as egui already does.
fn scrolling_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let ctx = ui.ctx().clone();
    // The scroll area keys its state by an `IdSalt`, not the bare string.
    let row_id = ui.make_persistent_id(egui::IdSalt::new(id));
    // egui's background drag of a scroll area is `id.with("area")`.
    let dragging = ctx.dragged_id() == Some(row_id.with("area"));
    let axis_key = row_id.with("axis");
    let mut axis = ctx
        .data(|d| d.get_temp::<DragAxis>(axis_key))
        .unwrap_or_default();
    let before = egui::scroll_area::State::load(&ctx, row_id);
    if dragging {
        // Travel is measured from where the finger landed, not by summing
        // deltas: a touch's first delta can jump from wherever the pointer
        // last was, which would lock the axis before the finger moved.
        let (origin, latest, delta) = ctx.input(|i| {
            (
                i.pointer.press_origin(),
                i.pointer.latest_pos(),
                i.pointer.delta(),
            )
        });
        match axis.vertical {
            None => {
                if let (Some(origin), Some(latest)) = (origin, latest) {
                    let travelled = latest - origin;
                    if travelled.length() >= AXIS_LOCK_DISTANCE {
                        let vertical = travelled.y.abs() > travelled.x.abs();
                        axis.vertical = Some(vertical);
                        if vertical {
                            forward_to_page(&ctx, -travelled.y);
                        }
                    }
                }
            }
            Some(true) => forward_to_page(&ctx, -delta.y),
            Some(false) => {}
        }
        ctx.data_mut(|d| d.insert_temp(axis_key, axis));
    } else {
        ctx.data_mut(|d| d.remove_temp::<DragAxis>(axis_key));
    }

    egui::ScrollArea::horizontal()
        .id_salt(id)
        .scroll_source(ScrollSource {
            drag: drag_scroll(),
            ..ScrollSource::default()
        })
        .show(ui, |ui| ui.horizontal(add));

    // egui has already moved the row by the drag's horizontal part this
    // frame; a vertical gesture holds it still, so put its offset back.
    if dragging && axis.vertical == Some(true) {
        if let Some(before) = before {
            if let Some(mut after) = egui::scroll_area::State::load(&ctx, row_id) {
                after.offset.x = before.offset.x;
                after.store(&ctx, row_id);
            }
        }
    }
}

/// Add `offset` to the page's scroll for its next frame (see
/// [`PAGE_FORWARD`]).
fn forward_to_page(ctx: &egui::Context, offset: f32) {
    ctx.data_mut(|d| *d.get_temp_mut_or_default::<f32>(egui::Id::new(PAGE_FORWARD)) += offset);
}

/// A row of captioned specimens (each one a [`wrapping_specimen`]) that
/// wraps onto new lines when the window is narrow. The gaps ride the
/// row's item spacing, never `add_space`: a space added after a line's
/// last tile pushes the row's extent past the window's edge, and every
/// later row then wraps that much wider.
///
/// `add` gets the item spacing the row's `Ui` had before the row set
/// its own, for each tile to restore inside itself: the row's spacing
/// is for between tiles, and a tile's art and caption keep the page's.
///
/// The row is `horizontal_wrapped` with its tiles aligned to the top of
/// the line, not its centre. `horizontal_wrapped` centres each widget on
/// the line laid out so far, and a tile (its height is not known when it
/// is placed) taller than that line settles lower than the one before it,
/// by half the difference: the tiles of a line then hang 10 to 100pt
/// apart instead of from one edge.
fn wrapping_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui, Vec2)) {
    let inherited = ui.spacing().item_spacing;
    // The size and the layout `horizontal_wrapped` starts a line with,
    // less its centre alignment.
    let line = Vec2::new(
        ui.available_size_before_wrap().x,
        ui.spacing().interact_size.y,
    );
    let layout = egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true);
    ui.allocate_ui_with_layout(line, layout, |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(spacing::SPACE_4, spacing::SPACE_3);
        add(ui, inherited);
    });
}

/// One captioned specimen in a [`wrapping_row`]: `art` above its
/// caption, in a top-down tile as wide as the wider of `art_width` and
/// the caption's line. egui's `horizontal_wrapped` moves a widget to
/// the next line only when it knows its size before it is laid out,
/// which a plain `ui.vertical` group never does (the row then runs on
/// past the window's edge); the tile allocates its width up front, so
/// the row wraps on a phone. `inherited` is the spacing the row hands
/// its tiles, which the tile lays its art and caption out with.
fn wrapping_specimen(
    ui: &mut egui::Ui,
    inherited: Vec2,
    theme: ThemeId,
    art_width: f32,
    caption: &str,
    art: impl FnOnce(&mut egui::Ui),
) {
    let job = specimen_job(
        caption,
        TypeStyle::Caption,
        theme,
        theme.colors().text_muted(),
        None,
    );
    let caption = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));
    let width = art_width.max(caption.size().x);
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing = inherited;
            art(ui);
            ui.add(egui::Label::new(caption));
        },
    );
}

/// Pair the option enums with their labels from `gallery_copy`: the
/// labels are copy, the enums are the form's state.
fn pairs<T: Copy>(values: &[T], labels: &[&'static str]) -> Vec<(T, &'static str)> {
    values
        .iter()
        .zip(labels)
        .map(|(value, label)| (*value, *label))
        .collect()
}

/// The radio flavour of [`pairs`]: `RadioOption` rows for a group.
fn radio_options<T: Copy>(values: &[T], labels: &[&'static str]) -> Vec<RadioOption<'static, T>> {
    values
        .iter()
        .zip(labels)
        .map(|(value, label)| option(*value, label))
        .collect()
}

/// The gallery's whole UI state and every panel. Constructed with
/// [`Gallery::new`], driven each frame by [`Gallery::chrome`].
pub struct Gallery {
    /// The live theme; read by the app crate to paint the viewport
    /// margins behind the safe areas.
    pub(crate) theme: ThemeId,
    /// The kit flags this host passes down, standing in for the web
    /// system's reduced-motion and reduced-transparency media queries;
    /// the host maps the OS reduce-motion preference onto them.
    pub(crate) flags: UiKitFlags,
    /// The practice-area filter chips' selected states, so the chips
    /// demo section toggles live.
    filters: [bool; 4],
    /// Whether the dismissible chip demo still holds its chip.
    saved_chip: bool,
    /// The project-details form the text inputs section edits.
    form: ProjectForm,
    /// The choice-controls demos' state (issue 06): the notification
    /// checkboxes, the practice-area children whose parent derives,
    /// and the local toggle's held value.
    choices: ChoiceDemos,
    /// The active gallery panel, driven by the tab strip and the
    /// bar's nav links alike.
    pub(crate) tab: usize,
    /// Test support, read only by the input tests: the page scroll
    /// area's id as of the last frame, under which egui keeps the
    /// page's live offset (the area's `scroll_area::State`). The app
    /// never reads it. A test cannot rebuild the id itself: egui derives
    /// it from the content panel's nested `Ui`s salted with
    /// `page_scroll_salt`, so a rebuild would re-trace the chrome's nesting.
    pub(crate) page_scroll_id: Option<egui::Id>,
    /// Test support, read only by the input tests: where the page
    /// scroll area showed its content as of the last frame (the
    /// scroll output's `inner_rect`, the visible page). The app never
    /// reads it.
    pub(crate) page_rect: Option<egui::Rect>,
    /// Which of the dismissible banner demos still hold their
    /// banners; the warning and the error stay until resolved.
    banners: BannerDemos,
    /// The snackbar host: one brief operational message at a time.
    snackbar: SnackbarHost,
    /// The bar's search field buffer.
    search: String,
    /// Whether the bar's search popover is open.
    search_open: bool,
    /// Whether the bar's theme menu is open.
    theme_open: bool,
}

/// The dismissible banner demos' state: the info and success banners
/// come back on demand; the warning and the error never leave on
/// their own.
struct BannerDemos {
    info: bool,
    success: bool,
}

impl BannerDemos {
    /// Both dismissible banners showing: the Feedback tab opens on all
    /// four tones, and the restore button waits for a dismissal.
    fn populated() -> Self {
        Self {
            info: true,
            success: true,
        }
    }
}

/// The choice-controls demos' state: the notification checkboxes, the
/// practice-area children (their parent's tri-state derives from
/// them), and the one local toggle's held value.
struct ChoiceDemos {
    weekly_digest: bool,
    blocked_builds: bool,
    areas: [bool; 3],
    remember_filters: bool,
}

impl ChoiceDemos {
    /// The Checkbox preview's own starting states: the digest on, the
    /// practice areas partly chosen, so the parent opens
    /// indeterminate.
    fn populated() -> Self {
        Self {
            weekly_digest: true,
            blocked_builds: false,
            areas: [true, false, true],
            remember_filters: false,
        }
    }

    /// The practice-area parent's state over its children, the
    /// preview's sync script.
    fn parent(&self) -> CheckboxState {
        CheckboxState::from_children(
            self.areas.len(),
            self.areas.iter().filter(|checked| **checked).count(),
        )
    }
}

/// A role name for display: strip the `md-sys-color-` prefix and set the
/// words in sentence case ("surface-container-low" becomes "Surface
/// container low").
fn sentence_case(role: &str) -> String {
    let stripped = role.strip_prefix("md-sys-color-").unwrap_or(role);
    let mut out = String::with_capacity(stripped.len());
    for (i, word) in stripped.split('-').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
        }
        out.push_str(chars.as_str());
    }
    out
}

/// The specimen colour each style sets, following the stylesheet's
/// helpers: headlines and pull-quotes in text-display, ledes and captions
/// in text-muted, body in text-body, the eyebrow in text-accent.
fn specimen_color(style: TypeStyle, colors: &ThemeColors) -> Color32 {
    match style {
        TypeStyle::Display
        | TypeStyle::Headline1
        | TypeStyle::Headline2
        | TypeStyle::Headline3
        | TypeStyle::Headline4
        | TypeStyle::Editorial => colors.text_display(),
        TypeStyle::Lede | TypeStyle::Caption => colors.text_muted(),
        TypeStyle::Body | TypeStyle::Mono => colors.text_body(),
        TypeStyle::Eyebrow => colors.text_accent,
    }
}

/// A tracking value as the specimen sets it: "+0.24em", "0em", or the
/// typographic minus for the headlines' negative tracks.
fn format_tracking(tracking_em: f32) -> String {
    if tracking_em == 0.0 {
        "0em".to_owned()
    } else if tracking_em < 0.0 {
        format!("\u{2212}{}em", -tracking_em)
    } else {
        format!("+{tracking_em}em")
    }
}

/// A style's leading as the specimen states it: the unitless ratio
/// ("1.02"), or, for the eyebrow, the tokens' absolute line ("16px").
/// The eyebrow's ratio is stored as 16 over 11 so its line box comes out
/// at 16px, and printed as a ratio it reads "1.4545455".
fn format_leading(style: TypeStyle) -> String {
    if style == TypeStyle::Eyebrow {
        format!("{}px", trim_float(style.line_height().round()))
    } else {
        trim_float(style.leading())
    }
}

/// The type specimen: every style in the active theme's faces, each with
/// its meta line (size over leading, tracking, family).
fn type_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::TYPE_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    for (style, sample) in copy::SPECIMEN {
        let color = specimen_color(*style, &colors);
        if *style == TypeStyle::Eyebrow {
            ui.add(TrackedLabel::new(
                sample,
                style.font_id(theme),
                style.tracking(),
                color,
            ));
        } else {
            ui.add(egui::Label::new(specimen_job(
                sample,
                *style,
                theme,
                color,
                Some(ui.available_width()),
            )));
        }
        ui.add_space(spacing::SPACE_1);
        let meta = format!(
            "{} · {}/{} · {} · {}",
            sentence_case(style.name().trim_start_matches("type-")),
            trim_float(style.size()),
            format_leading(*style),
            format_tracking(style.tracking()),
            style.family_name(theme),
        );
        ui.add(egui::Label::new(specimen_job(
            &meta,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_4);
    }
}

/// A float without its trailing zeros: 96.0 renders "96", 1.08 "1.08".
fn trim_float(value: f32) -> String {
    let formatted = format!("{value}");
    formatted
        .strip_suffix(".0")
        .unwrap_or(&formatted)
        .to_owned()
}

/// One state specimen: a button-shaped rect at `scale`, its caption,
/// and the focus ring when the state is focus.
///
/// The rect is as wide as the label needs, not a fixed width: the label
/// is laid out first and the rect takes the kit button's horizontal
/// padding around it, so a wider face (the themes' body faces differ in
/// width) widens the tile instead of spilling the label's ends past the
/// fill. The row scrolls sideways, so a wider tile costs nothing.
fn state_tile(
    ui: &mut egui::Ui,
    theme: ThemeId,
    caption: &str,
    fill: Color32,
    scale: f32,
    text: Color32,
    focused: bool,
) {
    let colors = theme.colors();
    let label = ui.painter().layout_no_wrap(
        copy::START_A_PROJECT.to_owned(),
        TypeStyle::Body.font_id(theme),
        text,
    );
    let width = button::min_width(ButtonSize::Default, label.size().x, false, false);
    let size = Vec2::new(width, spacing::SPACE_10);
    ui.vertical(|ui| {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let body = Rect::from_center_size(rect.center(), size * scale);
        let radius = CornerRadius::same(radii::RADIUS_MD as u8);
        ui.painter().rect_filled(body, radius, fill);
        if focused {
            states::focus_ring_at(ui.painter(), rect, radius, theme);
        }
        ui.painter()
            .galley(body.center() - label.size() / 2.0, label, text);
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            caption,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(ui.available_width()),
        )));
    });
}

/// The interaction states: hover, press, focus and disabled as
/// specimens beside a control that really takes focus via Tab.
fn states_section(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::STATES_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    let rest = colors.primary;
    let hover = states::composite_over(states::hover_overlay(colors.on_primary), rest);
    let pressed = states::composite_over(states::pressed_overlay(colors.on_primary), rest);
    let disabled = states::disabled(colors.on_surface, colors.surface_container);

    // The captions outsize their tiles, so the row scrolls rather
    // than clip on a narrow window.
    scrolling_row(ui, "states-tiles", |ui| {
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[0],
            rest,
            1.0,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[1],
            hover,
            1.0,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[2],
            pressed,
            states::PRESS_SCALE,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[3],
            rest,
            1.0,
            colors.on_primary,
            true,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[4],
            disabled.container,
            1.0,
            disabled.content,
            false,
        );
    });

    ui.add_space(spacing::SPACE_2);
    FocusRing::new().wrap(ui, theme, flags, |ui| {
        // A compact pill rather than egui's 18pt default, which leaves
        // the label touching its outline: at least SPACE_6 tall, with
        // SPACE_1 of air between the label and the outline's inner edge
        // above and below, and SPACE_2 beside it. The padding counts
        // the outline, which egui draws inside the button's rect.
        ui.scope(|ui| {
            let outline = ui.visuals().widgets.inactive.bg_stroke.width;
            ui.spacing_mut().button_padding =
                Vec2::new(spacing::SPACE_2, spacing::SPACE_1 + outline);
            ui.add(
                egui::Button::new(copy::STATES_TAB_BUTTON)
                    .min_size(Vec2::new(0.0, spacing::SPACE_6)),
            )
        })
        .inner
    });
    ui.add(egui::Label::new(specimen_job(
        copy::STATES_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The foundations as specimens: the spacing scale, all eight radii and
/// the active theme's five elevation levels.
fn foundations_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::FOUNDATIONS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    ui.label(copy::SPACING_LABEL);
    ui.add_space(spacing::SPACE_2);
    wrapping_row(ui, |ui, inherited| {
        for (name, value) in spacing::ALL {
            // The bar runs the step's own length; the widest steps
            // widen their tile to hold it.
            let width = spacing::SPACE_24.max(*value);
            let caption = format!("{name} · {}", trim_float(*value));
            wrapping_specimen(ui, inherited, theme, width, &caption, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(width, spacing::SPACE_2), Sense::hover());
                if *value > 0.0 {
                    let bar = Rect::from_min_size(rect.min, Vec2::new(*value, rect.height()));
                    ui.painter()
                        .rect_filled(bar, CornerRadius::same(0), colors.primary);
                }
            });
        }
    });

    ui.add_space(spacing::SPACE_4);
    ui.label(copy::RADII_LABEL);
    ui.add_space(spacing::SPACE_2);
    // Eight labelled swatches run wide; the row scrolls.
    scrolling_row(ui, "radii-row", |ui| {
        // The swatch's 1px outline is drawn outside its rect, and the
        // scroll area clips at the row's left edge: start the first
        // swatch one outline in, so its left pixel is inside the clip.
        ui.add_space(spacing::SPACE_PX);
        for (name, value) in radii::ALL {
            ui.vertical(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::splat(spacing::SPACE_16), Sense::hover());
                let radius = CornerRadius::same(value.min(255.0) as u8);
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    egui::Stroke::new(1.0, colors.outline_variant),
                    egui::StrokeKind::Outside,
                );
                ui.painter()
                    .rect_filled(rect.shrink(1.0), radius, colors.surface_container_high);
                // One line, however far the row has scrolled: a `Label`
                // wraps a job at the width left to it, and a swatch at the
                // row's right edge has little, so "Radius Lg · 12" broke
                // after "Radius" until it was scrolled in.
                ui.add(
                    egui::Label::new(specimen_job(
                        &format!("{} · {}", sentence_case(name), trim_float(*value)),
                        TypeStyle::Caption,
                        theme,
                        colors.text_muted(),
                        None,
                    ))
                    .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
            ui.add_space(spacing::SPACE_2);
        }
    });

    ui.add_space(spacing::SPACE_4);
    ui.label(format!("{} · {}", copy::ELEVATION_LABEL, theme.name()));
    ui.add_space(spacing::SPACE_2);
    let shadows = theme.shadows();
    let levels = [
        ("L1", shadows.l1),
        ("L2", shadows.l2),
        ("L3", shadows.l3),
        ("L4", shadows.l4),
        ("L5", shadows.l5),
    ];
    // Five 96px tiles run past a phone's width; they wrap.
    wrapping_row(ui, |ui, inherited| {
        for (level, layers) in levels {
            let caption = layers
                .first()
                .map(|layer| {
                    format!(
                        "{} · {} {} {} · {:.0}%",
                        level,
                        trim_float(layer.x),
                        trim_float(layer.y),
                        trim_float(layer.blur),
                        layer.color.a() as f32 * 100.0 / 255.0,
                    )
                })
                .unwrap_or_else(|| level.to_owned());
            wrapping_specimen(ui, inherited, theme, spacing::SPACE_24, &caption, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(spacing::SPACE_24, spacing::SPACE_24),
                    Sense::hover(),
                );
                let card = Rect::from_center_size(
                    rect.center(),
                    Vec2::new(spacing::SPACE_16, spacing::SPACE_10),
                );
                let radius = CornerRadius::same(radii::RADIUS_LG as u8);
                ui.painter().add(egui_shadow(layers).as_shape(card, radius));
                ui.painter()
                    .rect_filled(card, radius, colors.surface_container_low);
            });
        }
    });
}

/// The icons: the five token sizes, then the whole catalogue grouped by
/// its categories, the fill-capable icons shown in their filled cut
/// (filled is the selected or toggled state; everything else rests
/// outlined).
fn icons_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::ICONS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    wrapping_row(ui, |ui, inherited| {
        for size in IconSize::ALL {
            let caption = format!(
                "{} · {}",
                trim_float(size.px()),
                size.name().trim_start_matches("icon-size-")
            );
            wrapping_specimen(ui, inherited, theme, size.px(), &caption, |ui| {
                icon(ui, Icon::Search, *size, colors.primary);
            });
        }
    });
    ui.add(egui::Label::new(specimen_job(
        copy::ICONS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    for (category, icons) in CATEGORY_GROUPS {
        ui.add_space(spacing::SPACE_4);
        ui.label(format!("{category} · {}", icons.len()));
        ui.add_space(spacing::SPACE_1);
        // Equal cells, at least ICON_CELL_MIN wide: the columns come from
        // the width, so the catalogue lines up on any phone or window.
        Grid::new(ICON_CELL_MIN).show(ui, icons, |ui, _, one, size| {
            if one.has_fill() {
                icon_active(ui, *one, IconSize::Lg, colors.primary);
            } else {
                icon(ui, *one, IconSize::Lg, colors.primary);
            }
            let caption = if one.has_fill() {
                format!("{} · fills", one.name())
            } else {
                one.name().to_owned()
            };
            ui.add(egui::Label::new(specimen_job(
                &caption,
                TypeStyle::Caption,
                theme,
                colors.text_muted(),
                Some(size.x),
            )));
        });
    }
}

/// The narrowest icon cell: three columns on a 360pt phone, four on a
/// desktop window, each caption wrapping inside its cell.
const ICON_CELL_MIN: f32 = 96.0;

/// One logo band: a filled ground carrying the logo in the placement's
/// ink, with its caption.
fn logo_band(
    ui: &mut egui::Ui,
    theme: ThemeId,
    caption: &str,
    ground: Option<Color32>,
    add: impl FnOnce(&mut egui::Ui),
) {
    let colors = theme.colors();
    let mut frame = egui::Frame::default()
        .inner_margin(spacing::SPACE_4)
        .corner_radius(radii::RADIUS_MD as u8);
    if let Some(ground) = ground {
        frame = frame.fill(ground);
    }
    frame.show(ui, |ui| ui.horizontal(|ui| add(ui)));
    ui.add(egui::Label::new(specimen_job(
        caption,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The logo: the mark and wordmark on the canvas, reversed on the
/// primary and on an inverse band, the accent moment, and the size
/// floors in the caption.
fn logo_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::LOGO_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    logo_band(ui, theme, copy::LOGO_ON_CANVAS, None, |ui| {
        ui.add(Logo::mark().height(32.0));
        ui.add_space(spacing::SPACE_6);
        ui.add(Logo::wordmark());
    });
    ui.add_space(spacing::SPACE_2);
    logo_band(
        ui,
        theme,
        copy::LOGO_ON_PRIMARY,
        Some(colors.primary),
        |ui| {
            ui.add(
                Logo::mark()
                    .height(32.0)
                    .ink(ui_kit::logo::LogoInk::OnPrimary),
            );
            ui.add_space(spacing::SPACE_6);
            ui.add(Logo::wordmark().ink(ui_kit::logo::LogoInk::OnPrimary));
        },
    );
    ui.add_space(spacing::SPACE_2);
    logo_band(
        ui,
        theme,
        copy::LOGO_ON_INVERSE,
        Some(colors.inverse_surface),
        |ui| {
            ui.add(
                Logo::mark()
                    .height(32.0)
                    .ink(ui_kit::logo::LogoInk::Inverse),
            );
            ui.add_space(spacing::SPACE_6);
            ui.add(Logo::wordmark().ink(ui_kit::logo::LogoInk::Inverse));
        },
    );
    ui.add_space(spacing::SPACE_2);
    logo_band(ui, theme, copy::LOGO_ACCENT_MOMENT, None, |ui| {
        ui.add(Logo::mark().height(32.0).ink(ui_kit::logo::LogoInk::Accent));
    });
    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        copy::LOGO_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

// ---- the surface components (issue 08) ------------------------------------

/// A card variant specimen: the class name in mono, one line of what
/// the variant is, per the Card preview.
fn card_specimen(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    class: &str,
    description: &str,
    card: Card,
) {
    let colors = theme.colors();
    card.ui(ui, theme, flags, |ui| {
        // The preview's 256px content, narrower when the window is: the
        // large card's 32px padding would push it past a 360pt phone.
        ui.set_width((spacing::SPACE_40 + spacing::SPACE_24).min(ui.available_width()));
        ui.add(egui::Label::new(specimen_job(
            class,
            TypeStyle::Mono,
            theme,
            colors.text_body(),
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            description,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(ui.available_width()),
        )));
    });
}

/// The card variants in the Card preview's two-column grid: elevated
/// (the default), outlined, filled and interactive, the interactive
/// one a real control to hover and tab to, and the large padding step.
/// Phone widths stack them one to a row, as the form grids stack.
fn cards_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    ui.label(copy::CARDS_LABEL);
    ui.add_space(spacing::SPACE_2);
    let cards = [
        (
            copy::CARD_DEFAULT_CLASS,
            copy::CARD_DEFAULT_DESC,
            Card::new(),
        ),
        (
            copy::CARD_OUTLINED_CLASS,
            copy::CARD_OUTLINED_DESC,
            Card::new().outlined(),
        ),
        (
            copy::CARD_FILLED_CLASS,
            copy::CARD_FILLED_DESC,
            Card::new().filled().compact(),
        ),
        (
            copy::CARD_INTERACTIVE_CLASS,
            copy::CARD_INTERACTIVE_DESC,
            Card::new().interactive(),
        ),
        (
            copy::CARD_LARGE_CLASS,
            copy::CARD_LARGE_DESC,
            Card::new().large(),
        ),
    ];
    if ui.available_width() < NARROW_WIDTH {
        for (index, (class, description, card)) in cards.into_iter().enumerate() {
            if index > 0 {
                ui.add_space(spacing::SPACE_5);
            }
            card_specimen(ui, theme, flags, class, description, card);
        }
    } else {
        // The preview's rows: the default alone, the outlined and the
        // filled side by side, then the interactive and the large.
        egui::Grid::new("surfaces-cards")
            .num_columns(2)
            .spacing([spacing::SPACE_5, spacing::SPACE_5])
            .show(ui, |ui| {
                for row in [&cards[..1], &cards[1..3], &cards[3..4], &cards[4..]] {
                    for (class, description, card) in row {
                        card_specimen(ui, theme, flags, class, description, *card);
                    }
                    ui.end_row();
                }
            });
    }
}

/// The metric trio: one specific, sourced number per card, per the
/// MetricCard preview's own copy.
fn metrics_group(ui: &mut egui::Ui, theme: ThemeId) {
    ui.label(copy::METRICS_LABEL);
    ui.add_space(spacing::SPACE_2);
    // Three cards side by side run near the page width; the row
    // scrolls on a narrower window.
    scrolling_row(ui, "metrics-row", |ui| {
        MetricCard::new(
            copy::METRIC_FLEET_LABEL,
            copy::METRIC_FLEET_FIGURE,
            copy::METRIC_FLEET_SOURCE,
        )
        .ui(ui, theme);
        ui.add_space(spacing::SPACE_5);
        MetricCard::new(
            copy::METRIC_TENURE_LABEL,
            copy::METRIC_TENURE_FIGURE,
            copy::METRIC_TENURE_SOURCE,
        )
        .ui(ui, theme);
        ui.add_space(spacing::SPACE_5);
        MetricCard::new(
            copy::METRIC_SENIOR_LABEL,
            copy::METRIC_SENIOR_FIGURE,
            copy::METRIC_SENIOR_SOURCE,
        )
        .unit(copy::METRIC_SENIOR_UNIT)
        .ui(ui, theme);
    });
}

/// The editorial row: a case study, a field note and an open role,
/// image areas as the placeholder tiles until photography exists.
fn editorial_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    ui.label(copy::EDITORIAL_LABEL);
    ui.add_space(spacing::SPACE_2);
    scrolling_row(ui, "editorial-row", |ui| {
        EditorialCard::new(
            copy::EDITORIAL_CASE_EYEBROW,
            copy::EDITORIAL_CASE_HEADLINE,
            copy::EDITORIAL_CASE_BODY,
        )
        .action(copy::EDITORIAL_CASE_ACTION)
        .ui(ui, theme, flags);
        ui.add_space(spacing::SPACE_5);
        EditorialCard::new(
            copy::EDITORIAL_FIELD_EYEBROW,
            copy::EDITORIAL_FIELD_HEADLINE,
            copy::EDITORIAL_FIELD_BODY,
        )
        .action(copy::EDITORIAL_FIELD_ACTION)
        .ui(ui, theme, flags);
        ui.add_space(spacing::SPACE_5);
        EditorialCard::new(
            copy::EDITORIAL_ROLE_EYEBROW,
            copy::EDITORIAL_ROLE_HEADLINE,
            copy::EDITORIAL_ROLE_BODY,
        )
        .action(copy::EDITORIAL_ROLE_ACTION)
        .ui(ui, theme, flags);
    });
}

/// One glass demo tile: a scene of palette content (display type, a
/// tertiary dot, a primary block on the canvas scenes) with a pane over
/// its lower part, carrying the pane's name, spec and use. The scene
/// supplies the backdrop the tint and its opacity play against.
struct GlassScene {
    /// The scene's ground: `surface-container-high` for the material
    /// tiles, the page canvas for the tint tiles.
    fill: Color32,
    /// Whether the scene carries the primary block (the canvas scenes).
    with_block: bool,
}

/// The tile's pane: which pane to paint and what to call it.
struct GlassPaneSpec {
    thickness: GlassThickness,
    tint: GlassTint,
    name: String,
}

/// The type-specimen glyph the glass scenes set behind their panes:
/// display-scale letters for the tint to play against.
const GLASS_SCENE_GLYPH: &str = "Aa";

/// Paint one glass demo tile: the scene, then the pane over its lower
/// part, chrome radius md, through the recipe so the specular edge and
/// the L2 shadow ride along.
fn glass_tile(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    width: f32,
    scene: GlassScene,
    pane: GlassPaneSpec,
) {
    let colors = theme.colors();
    let height = 212.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(radii::RADIUS_LG as u8);
    ui.painter().rect_filled(rect, radius, scene.fill);
    // The scene's art: display type and a dot, the palette content the
    // web preview scrolls under its panes.
    ui.painter().text(
        rect.left_top() + Vec2::new(spacing::SPACE_2, -spacing::SPACE_1),
        Align2::LEFT_TOP,
        GLASS_SCENE_GLYPH,
        egui::FontId {
            size: 128.0,
            family: TypeStyle::Display.family(theme),
        },
        colors.primary,
    );
    ui.painter().circle_filled(
        rect.right_top() + Vec2::new(-spacing::SPACE_4, spacing::SPACE_8),
        spacing::SPACE_8,
        colors.tertiary,
    );
    if scene.with_block {
        ui.painter().rect_filled(
            Rect::from_min_size(
                rect.right_top(),
                Vec2::new(spacing::SPACE_12, spacing::SPACE_10),
            ),
            CornerRadius::same(0),
            colors.primary,
        );
    }
    let pane_rect = Rect::from_min_max(
        rect.left_bottom() + Vec2::new(spacing::SPACE_2, -96.0),
        rect.right_bottom() - Vec2::new(spacing::SPACE_2, spacing::SPACE_2),
    );
    let recipe = glass_recipe(pane.thickness, pane.tint, theme, flags);
    paint_glass(ui.painter(), pane_rect, radii::RADIUS_MD as u8, &recipe);
    let text = pane_rect.left_top() + Vec2::new(spacing::SPACE_3, spacing::SPACE_3);
    let line = spacing::SPACE_3 + 14.0;
    ui.painter().text(
        text,
        Align2::LEFT_TOP,
        pane.name,
        egui::FontId {
            size: 14.0,
            family: TypeStyle::Headline4.family(theme),
        },
        recipe.label,
    );
    ui.painter().text(
        text + Vec2::new(0.0, line),
        Align2::LEFT_TOP,
        format!(
            "blur {} · sat {:.0}%",
            pane.thickness.blur_px(),
            pane.thickness.saturation_percent()
        ),
        egui::FontId {
            size: 12.0,
            family: TypeStyle::Mono.family(theme),
        },
        recipe.label,
    );
    ui.painter().text(
        text + Vec2::new(0.0, line * 2.0),
        Align2::LEFT_TOP,
        pane.thickness.usage(),
        egui::FontId {
            size: 12.0,
            family: TypeStyle::Caption.family(theme),
        },
        recipe.label,
    );
}

/// The glass panes: the five thicknesses over palette content, then the
/// three tints beside the reduced-transparency fallback, one live pane
/// laid out as chrome, and the usage rules in the caption.
fn glass_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.label(copy::GLASS_LABEL);
    ui.add_space(spacing::SPACE_2);
    // Five panes with their captions run past the page width; the
    // row scrolls.
    scrolling_row(ui, "glass-thicknesses", |ui| {
        for thickness in GlassThickness::ALL {
            glass_tile(
                ui,
                theme,
                flags,
                176.0,
                GlassScene {
                    fill: colors.surface_container_high,
                    with_block: false,
                },
                GlassPaneSpec {
                    thickness: *thickness,
                    tint: GlassTint::Surface,
                    name: sentence_case(thickness.name()),
                },
            );
            ui.add_space(spacing::SPACE_3);
        }
    });

    ui.add_space(spacing::SPACE_2);
    ui.label(copy::GLASS_TINTS_LABEL);
    ui.add_space(spacing::SPACE_1);
    let solid_flags = UiKitFlags {
        reduced_transparency: true,
        ..Default::default()
    };
    let tints: &[(GlassTint, &str, &UiKitFlags)] = &[
        (GlassTint::Surface, copy::GLASS_SURFACE_CLASS, flags),
        (GlassTint::Primary, copy::GLASS_PRIMARY_CLASS, flags),
        (GlassTint::Accent, copy::GLASS_ACCENT_CLASS, flags),
        (GlassTint::Surface, copy::GLASS_FALLBACK_NAME, &solid_flags),
    ];
    scrolling_row(ui, "glass-tints", |ui| {
        for (tint, name, pane_flags) in tints {
            glass_tile(
                ui,
                theme,
                pane_flags,
                224.0,
                GlassScene {
                    fill: colors.background,
                    with_block: true,
                },
                GlassPaneSpec {
                    thickness: GlassThickness::Regular,
                    tint: *tint,
                    name: (*name).to_owned(),
                },
            );
            ui.add_space(spacing::SPACE_3);
        }
    });

    // One live pane, laid out as chrome with content in it; the host's
    // reduced-transparency checkbox flips it solid in place.
    ui.add_space(spacing::SPACE_2);
    let recipe = LiquidGlass::new().recipe(theme, flags);
    LiquidGlass::new().ui(ui, theme, flags, |ui| {
        ui.set_width(spacing::SPACE_40 + spacing::SPACE_10);
        ui.add(egui::Label::new(
            egui::RichText::new(copy::GLASS_CHROME_LABEL)
                .color(recipe.label)
                .size(14.0)
                .family(TypeStyle::Headline4.family(theme)),
        ));
    });

    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        copy::GLASS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

// ---- the action components (issue 05) --------------------------------------

/// The buttons group: the four emphases in the Button preview's own
/// copy, the three sizes, and the icon variants, every one a live
/// control (hover, hold, Tab) with the disabled and busy pair.
fn buttons_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.label(copy::BUTTONS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        Button::new(copy::START_A_PROJECT).ui(ui, theme, flags);
        Button::new(copy::BUTTON_READ_CASE_STUDY)
            .emphasis(ButtonEmphasis::Secondary)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_HIRE_A_TEAM)
            .emphasis(ButtonEmphasis::Tertiary)
            .icon_trailing(Icon::ArrowForward)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_CANCEL)
            .emphasis(ButtonEmphasis::Ghost)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_DISABLED)
            .disabled()
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_BUSY).busy().ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::BUTTONS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.label(copy::SIZES_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        Button::new(copy::BUTTON_OPEN_DASHBOARD)
            .large()
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_SAVE_CHANGES).ui(ui, theme, flags);
        Button::new(copy::BUTTON_FILTER_RESULTS)
            .small()
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::SIZES_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.label(copy::BUTTON_ICONS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        Button::new(copy::BUTTON_SEARCH_ARCHIVE)
            .icon_leading(Icon::Search)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_GET_PROPOSAL)
            .icon_trailing(Icon::ArrowForward)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_VISIT_STUDIO)
            .icon_trailing(Icon::ArrowOutward)
            .ui(ui, theme, flags);
        Button::new("")
            .icon_only(Icon::Search, copy::BUTTON_SEARCH_EVERYTHING)
            .ui(ui, theme, flags);
        Button::new("")
            .icon_only(Icon::ArrowOutward, copy::BUTTON_OPEN_EXTERNAL)
            .emphasis(ButtonEmphasis::Secondary)
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::BUTTON_ICONS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.add(egui::Label::new(specimen_job(
        copy::TERTIARY_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The chips group: the preview's practice-area filter row as live
/// toggles, the featured label, and a dismissible chip reporting its
/// typed event.
fn chips_group(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    filters: &mut [bool; 4],
    saved: &mut bool,
) {
    let colors = theme.colors();
    ui.label(copy::CHIPS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.add(TrackedLabel::new(
        copy::CHIPS_PRACTICE_AREA,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_1);
    ui.horizontal_wrapped(|ui| {
        let last_filter = copy::CHIP_FILTERS.len() - 1;
        for (index, (label, selected)) in copy::CHIP_FILTERS
            .iter()
            .zip(filters.iter_mut())
            .enumerate()
        {
            if index == last_filter {
                // The featured chip stands a further `space-4` from the
                // filters. The gap rides the row's item spacing, which
                // follows the last filter chip: an `add_space` after it
                // does not wrap, so a chip ending within the gap of the
                // page's right edge left the space past it, and the page
                // grew that wide (see `wrapping_row`).
                ui.spacing_mut().item_spacing.x += spacing::SPACE_4;
            }
            if let Some(ChipEvent::Toggled(now)) = Chip::new(label)
                .set_selected(*selected)
                .ui(ui, theme, flags)
                .event
            {
                *selected = now;
            }
        }
        Chip::new(copy::CHIP_FEATURED).accent().ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::CHIPS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.horizontal(|ui| {
        if *saved {
            if let Some(event) = Chip::new(copy::CHIP_CLIENT_WORK)
                .dismissible()
                .ui(ui, theme, flags)
                .event
            {
                if event == ChipEvent::Dismissed {
                    *saved = false;
                }
            }
        } else {
            if Button::new(copy::CHIP_RESTORE)
                .emphasis(ButtonEmphasis::Ghost)
                .ui(ui, theme, flags)
                .clicked
            {
                *saved = true;
            }
        }
    });
    ui.add(egui::Label::new(specimen_job(
        copy::CHIP_DISMISS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The badges group: the six tones in the Badge preview's own words,
/// and the count badges on their 44px icon buttons.
fn badges_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.label(copy::BADGES_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        for (label, tone) in copy::BADGES {
            Badge::new(label, *tone).ui(ui, theme);
        }
    });
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        IconButton::new(copy::BADGE_NOTIFICATIONS, Icon::Notifications)
            .count(3)
            .ui(ui, theme, flags);
        IconButton::new(copy::BADGE_MAIL, Icon::Mail)
            .count(120)
            .ui(ui, theme, flags);
        IconButton::new(copy::BADGE_MESSAGES, Icon::Chat)
            .activity_dot()
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::BADGES_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The actions section: every Button, Chip and Badge variant as live,
/// Tab-focusable instances in the active theme.
fn actions_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    filters: &mut [bool; 4],
    saved: &mut bool,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::ACTIONS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    buttons_group(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);
    chips_group(ui, theme, flags, filters, saved);
    ui.add_space(spacing::SPACE_4);
    badges_group(ui, theme, flags);
}

/// The components section: the surface components issue 08 adds, in
/// the preview's own copy.
fn components_section(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::SURFACES_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    cards_group(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);
    metrics_group(ui, theme);
    ui.add_space(spacing::SPACE_4);
    editorial_group(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);
    glass_group(ui, theme, flags);
}

// ---- the text input components (issue 07) ----------------------------------

/// The region options, the Radio preview's example domain, shared with
/// this section's select.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Nordics,
    Europe,
    Global,
}

const REGION_VALUES: &[Region] = &[Region::Nordics, Region::Europe, Region::Global];

/// The team sizes, the Select preview's own options.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TeamSize {
    OneToTwo,
    ThreeToFive,
    SixOrMore,
}

const TEAM_SIZE_VALUES: &[TeamSize] = &[
    TeamSize::OneToTwo,
    TeamSize::ThreeToFive,
    TeamSize::SixOrMore,
];

/// The delivery offices: nine of them, so the list scrolls inside the
/// eight rows the kit caps it at.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Office {
    Gothenburg,
    Stockholm,
    Copenhagen,
    Oslo,
    Helsinki,
    Berlin,
    Amsterdam,
    London,
    Edinburgh,
}

const OFFICE_VALUES: &[Office] = &[
    Office::Gothenburg,
    Office::Stockholm,
    Office::Copenhagen,
    Office::Oslo,
    Office::Helsinki,
    Office::Berlin,
    Office::Amsterdam,
    Office::London,
    Office::Edinburgh,
];

/// One invoice currency, locked by the contract: the disabled select.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Currency {
    Sek,
}

/// The project-details form the section edits, with the email's
/// touched flag carrying the validate-on-leave rule.
#[derive(Default)]
struct ProjectForm {
    name: String,
    email: String,
    reference: String,
    search: String,
    brief: String,
    /// Whether the person has left the email field yet: the field
    /// validates on leave, not while they type.
    email_touched: bool,
    region: Option<Region>,
    team_size: Option<TeamSize>,
    office: Option<Office>,
}

impl ProjectForm {
    fn populated() -> Self {
        Self {
            reference: copy::REFERENCE_DEFAULT.to_owned(),
            brief: copy::BRIEF_DEFAULT.to_owned(),
            region: Some(Region::Nordics),
            office: Some(Office::Gothenburg),
            ..Default::default()
        }
    }
}

/// A work email is valid when it has one @ with something before it and
/// a domain with a dot inside it: loose enough for the demo, strict
/// enough to be worth fixing.
fn email_is_valid(email: &str) -> bool {
    match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain
                    .split_once('.')
                    .is_some_and(|(head, tail)| !head.is_empty() && !tail.is_empty())
        }
        None => false,
    }
}

/// The form's left column: the project name and the email with its
/// validate-on-leave rule.
fn fields_left(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    TextField::new(copy::FIELD_NAME_LABEL, &mut form.name)
        .placeholder(copy::FIELD_NAME_PLACEHOLDER)
        .hint(copy::FIELD_NAME_HINT)
        .ui(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);

    // The email validates when the person leaves the field, not
    // while they type: the first leave arms it, and the error
    // clears as soon as the address is fixed.
    let invalid = form.email_touched && !email_is_valid(&form.email);
    let mut email = TextField::new(copy::FIELD_EMAIL_LABEL, &mut form.email)
        .placeholder(copy::FIELD_EMAIL_PLACEHOLDER)
        .hint(copy::FIELD_EMAIL_HINT);
    if invalid {
        email = email.error(copy::EMAIL_ERROR);
    }
    let email_field = email.ui(ui, theme, flags);
    if email_field.lost_focus {
        form.email_touched = true;
    }
}

/// The form's right column: the search field and the locked reference.
fn fields_right(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    TextField::new(copy::FIELD_SEARCH_LABEL, &mut form.search)
        .placeholder(copy::SEARCH_PLACEHOLDER)
        .leading_icon(Icon::Search)
        .ui(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);

    TextField::new(copy::FIELD_REFERENCE_LABEL, &mut form.reference)
        .disabled()
        .hint(copy::SET_BY_CONTRACT)
        .ui(ui, theme, flags);
}

/// The fields group: the project-details form's text fields, the
/// preview's own grid of variants as live controls. Wide windows set
/// the two columns side by side as the preview does; phone widths
/// stack them.
fn fields_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let colors = theme.colors();
    ui.label(copy::FIELDS_LABEL);
    ui.add_space(spacing::SPACE_2);
    if ui.available_width() < NARROW_WIDTH {
        fields_left(ui, theme, flags, form);
        ui.add_space(spacing::SPACE_4);
        fields_right(ui, theme, flags, form);
    } else {
        ui.columns(2, |columns| {
            let (left, right) = columns.split_at_mut(1);
            fields_left(&mut left[0], theme, flags, form);
            fields_right(&mut right[0], theme, flags, form);
        });
    }

    // The textarea, spanning both columns.
    ui.add_space(spacing::SPACE_4);
    TextField::new(copy::FIELD_BRIEF_LABEL, &mut form.brief)
        .placeholder(copy::FIELD_BRIEF_PLACEHOLDER)
        .multiline(3)
        .ui(ui, theme, flags);

    ui.add_space(spacing::SPACE_4);
    ui.add(egui::Label::new(specimen_job(
        copy::FIELDS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The selects' left column: the region and the delivery office.
fn selects_left(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let regions = pairs(REGION_VALUES, copy::REGIONS);
    let offices = pairs(OFFICE_VALUES, copy::OFFICES);
    let region = Select::new(copy::SELECT_REGION_LABEL)
        .options(&regions)
        .selected(form.region)
        .hint(copy::SELECT_REGION_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = region.committed {
        form.region = Some(picked);
    }
    ui.add_space(spacing::SPACE_4);

    let office = Select::new(copy::SELECT_OFFICE_LABEL)
        .options(&offices)
        .selected(form.office)
        .hint(copy::SELECT_OFFICE_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = office.committed {
        form.office = Some(picked);
    }
}

/// The selects' right column: the team size and the locked currency.
fn selects_right(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let teams = pairs(TEAM_SIZE_VALUES, copy::TEAM_SIZES);
    let currencies = pairs(&[Currency::Sek], copy::CURRENCIES);
    let team = Select::new(copy::SELECT_TEAM_LABEL)
        .options(&teams)
        .selected(form.team_size)
        .placeholder(copy::SELECT_TEAM_PLACEHOLDER)
        .hint(copy::SELECT_TEAM_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = team.committed {
        form.team_size = Some(picked);
    }
    ui.add_space(spacing::SPACE_4);

    Select::new(copy::SELECT_CURRENCY_LABEL)
        .options(&currencies)
        .selected(Some(Currency::Sek))
        .disabled()
        .hint(copy::SET_BY_CONTRACT)
        .ui(ui, theme, flags);
}

/// The selects group: the region with a choice made, the team size on
/// its placeholder, a nine-office list that scrolls, and the locked
/// currency. Wide windows pair the columns as the preview does; phone
/// widths stack them.
fn selects_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let colors = theme.colors();
    ui.label(copy::SELECTS_LABEL);
    ui.add_space(spacing::SPACE_2);
    if ui.available_width() < NARROW_WIDTH {
        selects_left(ui, theme, flags, form);
        ui.add_space(spacing::SPACE_4);
        selects_right(ui, theme, flags, form);
    } else {
        ui.columns(2, |columns| {
            let (left, right) = columns.split_at_mut(1);
            selects_left(&mut left[0], theme, flags, form);
            selects_right(&mut right[0], theme, flags, form);
        });
    }

    ui.add_space(spacing::SPACE_4);
    ui.add(egui::Label::new(specimen_job(
        copy::SELECTS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The text inputs section: a believable project-details form, every
/// field and select a live control in the active theme.
fn text_inputs_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    form: &mut ProjectForm,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::INPUTS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    fields_group(ui, theme, flags, form);
    ui.add_space(spacing::SPACE_4);
    selects_group(ui, theme, flags, form);
}

// ---- the choice controls (issue 06) ----------------------------------------

/// The start options, the Radio preview's invalid-group specimen. The
/// group carries no state: the error is the demo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Start {
    ThisQuarter,
    NextQuarter,
    Undecided,
}

const START_VALUES: &[Start] = &[Start::ThisQuarter, Start::NextQuarter, Start::Undecided];

/// One nested checkbox row at the preview's 28px indent, its typed
/// toggle handed back.
fn nested_checkbox_row(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    label: &str,
    checked: bool,
) -> Option<CheckboxState> {
    let mut toggled = None;
    ui.horizontal(|ui| {
        ui.add_space(
            ui_kit::components::checkbox::BOX_SIZE + ui_kit::components::checkbox::LABEL_GAP,
        );
        toggled = Checkbox::new(label)
            .checked(checked)
            .ui(ui, theme, flags)
            .toggled;
    });
    toggled
}

/// The checkboxes group: the practice areas with their tri-state
/// parent, and the notifications with the hint and disabled
/// specimens — the Checkbox preview's own two columns.
fn checkboxes_group(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    choices: &mut ChoiceDemos,
) {
    ui.label(copy::CHECKBOXES_LABEL);
    ui.add_space(spacing::SPACE_2);

    label_row(ui, theme, copy::CHECK_AREAS_LABEL);
    hint_row(ui, theme, copy::CHECK_AREAS_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(toggled) = Checkbox::new(copy::CHECK_PARENT_LABEL)
        .state(choices.parent())
        .ui(ui, theme, flags)
        .toggled
    {
        // The parent's click sets every child, the preview's script.
        choices.areas = [toggled.is_checked(); 3];
    }
    for (index, area) in copy::PRACTICE_AREAS.iter().enumerate() {
        ui.add_space(ui_kit::components::checkbox::ROW_GAP);
        if let Some(toggled) = nested_checkbox_row(ui, theme, flags, area, choices.areas[index]) {
            choices.areas[index] = toggled.is_checked();
        }
    }

    ui.add_space(spacing::SPACE_4);
    label_row(ui, theme, copy::CHECK_NOTIFICATIONS_LABEL);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(toggled) = Checkbox::new(copy::CHECK_DIGEST_LABEL)
        .checked(choices.weekly_digest)
        .ui(ui, theme, flags)
        .toggled
    {
        choices.weekly_digest = toggled.is_checked();
    }
    hint_row(ui, theme, copy::CHECK_DIGEST_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(toggled) = Checkbox::new(copy::CHECK_BUILDS_LABEL)
        .checked(choices.blocked_builds)
        .ui(ui, theme, flags)
        .toggled
    {
        choices.blocked_builds = toggled.is_checked();
    }
    hint_row(ui, theme, copy::CHECK_BUILDS_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    Checkbox::new(copy::CHECK_TEXTS_LABEL)
        .disabled()
        .ui(ui, theme, flags);
    hint_row(ui, theme, copy::CHECK_TEXTS_HINT);
}

/// The radio groups: the region, live against the same field the
/// select edits, and the invalid start group, the preview's error
/// specimen.
fn radios_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    ui.label(copy::RADIOS_LABEL);
    ui.add_space(spacing::SPACE_2);

    let regions = radio_options(REGION_VALUES, copy::REGIONS);
    let region = RadioGroup::new(copy::SELECT_REGION_LABEL)
        .options(&regions)
        .selected(form.region)
        .hint(copy::SELECT_REGION_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = region.committed {
        form.region = Some(picked);
    }

    ui.add_space(spacing::SPACE_4);
    let starts = radio_options(START_VALUES, copy::START_OPTIONS);
    RadioGroup::new(copy::RADIO_START_LABEL)
        .options(&starts)
        .error(copy::RADIO_START_ERROR)
        .ui(ui, theme, flags);
}

/// The toggles: the spec's own example use, reduced motion, wired to
/// the gallery's flags so flipping it changes every component at once;
/// a local setting; and the disabled pair at both positions.
fn toggles_group(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &mut UiKitFlags,
    choices: &mut ChoiceDemos,
) {
    ui.label(copy::TOGGLES_LABEL);
    ui.add_space(spacing::SPACE_2);

    if let Some(on) = Toggle::new(copy::REDUCED_MOTION)
        .checked(flags.reduced_motion)
        .ui(ui, theme, flags)
        .toggled
    {
        // The switch that is its own example: the flag the theme menu
        // also sets, honoured across the kit.
        flags.reduced_motion = on;
    }
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(on) = Toggle::new(copy::TOGGLE_FILTERS_LABEL)
        .checked(choices.remember_filters)
        .ui(ui, theme, flags)
        .toggled
    {
        choices.remember_filters = on;
    }
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    Toggle::new(copy::TOGGLE_SYNC_LABEL)
        .checked(true)
        .disabled()
        .ui(ui, theme, flags);
    hint_row(ui, theme, copy::TOGGLE_SYNC_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    Toggle::new(copy::TOGGLE_TIMESHEETS_LABEL)
        .checked(false)
        .disabled()
        .ui(ui, theme, flags);
    hint_row(ui, theme, copy::TOGGLE_TIMESHEETS_HINT);
}

/// The choice-controls section: the checkboxes, the radio groups and
/// the toggles, every control live in the active theme.
fn choice_controls_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &mut UiKitFlags,
    form: &mut ProjectForm,
    choices: &mut ChoiceDemos,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::CHOICE_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    checkboxes_group(ui, theme, flags, choices);
    ui.add_space(spacing::SPACE_4);
    radios_group(ui, theme, flags, form);
    ui.add_space(spacing::SPACE_4);
    toggles_group(ui, theme, flags, choices);

    ui.add_space(spacing::SPACE_4);
    ui.add(egui::Label::new(specimen_job(
        copy::CHOICE_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

// ---- the feedback components (issue 09) ------------------------------------

/// The banner demos: the four tones in the Banner preview's own copy,
/// the error's message carrying the four-part voice (what is wrong,
/// why it matters, what to do next, where to get help). Anchored at
/// the panel top, as the spec places them.
fn banners_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, demos: &mut BannerDemos) {
    let colors = theme.colors();
    ui.label(copy::BANNERS_LABEL);
    ui.add_space(spacing::SPACE_2);

    if demos.info {
        let response = Banner::new(
            BannerTone::Info,
            copy::BANNER_INFO_TITLE,
            copy::BANNER_INFO_MESSAGE,
        )
        .ui(ui, theme, flags);
        if response.event == Some(BannerEvent::Dismissed) {
            demos.info = false;
        }
    }
    if demos.success {
        let response = Banner::new(
            BannerTone::Success,
            copy::BANNER_SUCCESS_TITLE,
            copy::BANNER_SUCCESS_MESSAGE,
        )
        .ui(ui, theme, flags);
        if response.event == Some(BannerEvent::Dismissed) {
            demos.success = false;
        }
    }
    Banner::new(
        BannerTone::Warning,
        copy::BANNER_WARNING_TITLE,
        copy::BANNER_WARNING_MESSAGE,
    )
    .ui(ui, theme, flags);
    Banner::new(
        BannerTone::Error,
        copy::BANNER_ERROR_TITLE,
        copy::BANNER_ERROR_MESSAGE,
    )
    .ui(ui, theme, flags);

    if !demos.info || !demos.success {
        ui.add_space(spacing::SPACE_2);
        if Button::new(copy::BANNERS_RESTORE)
            .emphasis(ButtonEmphasis::Ghost)
            .ui(ui, theme, flags)
            .clicked
        {
            *demos = BannerDemos::populated();
        }
    }

    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        copy::BANNERS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The snackbar triggers: buttons whose actions enqueue the host's
/// brief operational messages, one at a time, near the bottom of the
/// window.
fn snackbars_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, host: &mut SnackbarHost) {
    let colors = theme.colors();
    ui.label(copy::SNACKBARS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        if Button::new(copy::SNACK_SAVE_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(Snackbar::success(copy::SNACK_SAVED));
        }
        if Button::new(copy::SNACK_COPY_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(Snackbar::info(copy::SNACK_LINK_COPIED));
        }
        if Button::new(copy::SNACK_SUBMIT_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(Snackbar::info(copy::SNACK_SUBMITTED).action(copy::SNACK_VIEW_ACTION));
        }
        if Button::new(copy::SNACK_ARCHIVE_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(
                Snackbar::success(copy::SNACK_ARCHIVED_TITLE)
                    .title(copy::SNACK_ARCHIVED_TITLE)
                    .line(copy::SNACK_ARCHIVED_LINE)
                    .action(copy::SNACK_UNDO_ACTION),
            );
        }
        if Button::new(copy::SNACK_SYNC_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(
                Snackbar::error(copy::SNACK_SYNC_FAILED_TITLE)
                    .title(copy::SNACK_SYNC_FAILED_TITLE)
                    .line(copy::SNACK_SYNC_FAILED_LINE)
                    .action(copy::SNACK_RETRY_ACTION),
            );
        }
    });
    ui.add(egui::Label::new(specimen_job(
        copy::SNACKBARS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The feedback section: the banners at the panel top, the snackbar
/// triggers below, the snackbar itself floating near the window's
/// bottom edge (painted by the host in `Gallery::ui`).
fn feedback_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    banners: &mut BannerDemos,
    snackbar: &mut SnackbarHost,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::FEEDBACK_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    banners_group(ui, theme, flags, banners);
    ui.add_space(spacing::SPACE_4);
    snackbars_group(ui, theme, flags, snackbar);
}

/// What the app bar holds in the room its row has. A wide window takes
/// everything: the wordmark, the nav links, the call to action, the
/// theme trigger naming the live theme, and the search. As the room
/// runs out the bar sheds, in this order, the theme's name (a nicety),
/// the call to action (a specimen: in the gallery it does nothing) and
/// the nav links (the Tabs strip beneath switches the same panels),
/// until what is left fits; on the narrowest phones the theme trigger
/// last gives up its label and keeps its icon. The theme menu and the
/// search, the bar's working controls, always stay. The order only
/// runs one way, so a narrower window never holds more than a wider
/// one, and the compact bars (the nav links shed) are the phone's.
struct BarFit {
    /// The nav links follow the wordmark.
    nav: bool,
    /// The call to action ends the actions, flush right.
    cta: bool,
    /// How the theme trigger reads.
    trigger: ThemeTrigger,
}

/// How the bar's theme trigger reads, fullest first.
enum ThemeTrigger {
    /// The menu's label followed by the live theme's name.
    Named,
    /// The menu's label alone.
    Bare,
    /// The kit's icon button with the tune icon, named for a screen
    /// reader by the menu's label.
    Icon,
}

impl ThemeTrigger {
    /// How far the trigger's box runs past the row's right edge, when
    /// it ends the row, so that its icon stands the bar's padding in
    /// from the pane's edge. The box holds its icon's room inside it:
    /// the default button's horizontal padding for the labelled
    /// triggers, and for the icon button the room between its 44px
    /// circle and its icon. The icon is measured by its box, not by its
    /// ink, which sits a further 5px in for the chevron and 3px for the
    /// tune icon.
    ///
    /// The labelled triggers pull back from that by
    /// [`states::FOCUS_RING_REACH`], a deliberate trade-off.
    /// Overhanging by the whole padding put their box 4px from the
    /// pane's edge, and their focus ring, which reaches
    /// [`states::FOCUS_RING_REACH`] past the box, on the bar's border,
    /// where it is cut. Now the ring ends where the box would have, 4px
    /// in from the edge (`BAR_PAD_X` less the padding), and the icon's
    /// box stands 4px further in than the wordmark (28px against 24px):
    /// the bar's two ends are unequal, a known limit of the bar.
    ///
    /// That 4px of clearance is `BAR_PAD_X` less the padding whatever
    /// the reach is; it is also the reach, which the focus-ring test
    /// asks of it, only while both are 4px. The icon button and the
    /// call to action keep their rings clear without a pull back.
    fn overhang(&self) -> f32 {
        match self {
            ThemeTrigger::Named | ThemeTrigger::Bare => {
                ButtonSize::Default.padding_x() - states::FOCUS_RING_REACH
            }
            ThemeTrigger::Icon => {
                (ui_kit::components::badge::ICON_BUTTON_SIZE - IconSize::Lg.px()) / 2.0
            }
        }
    }
}

impl BarFit {
    /// The fullest bar that fits the row `ui` is about to lay out. The
    /// widths are measured before anything is placed, in the widgets'
    /// own faces and with the bar's own gaps: the row runs left to
    /// right, so by the time the actions are laid out the nav links
    /// are already down, and an action without room lands on top of
    /// them. `narrow`, the phone layout, goes straight to the compact
    /// bars.
    fn measure(ui: &egui::Ui, theme: ThemeId, narrow: bool) -> BarFit {
        // egui spaces each widget from the next by its item spacing,
        // and the bar's own gaps come on top of that.
        let spacing = ui.spacing().item_spacing.x;
        let action = |width: f32| width + spacing + ACTIONS_GAP;
        let search = ui_kit::components::badge::ICON_BUTTON_SIZE;
        let trigger_width =
            |trigger: ThemeTrigger| action(trigger_button_width(ui, theme, &trigger));
        let row = ui.available_width();
        let logo = Logo::wordmark().height(LOGO_HEIGHT).size().x;
        if !narrow {
            let nav: f32 = copy::TABS
                .iter()
                .map(|label| nav_link_width(ui, theme, label) + spacing + LINK_GAP)
                .sum();
            let cta = action(cta_button_width(ui, theme));
            let named = trigger_width(ThemeTrigger::Named);
            let bare = trigger_width(ThemeTrigger::Bare);
            // The room left once the wordmark, with the gap after it,
            // and the search, which every bar holds, are in.
            let room = row - (logo + spacing + GROUP_GAP) - search;
            let fuller = [
                (
                    BarFit {
                        nav: true,
                        cta: true,
                        trigger: ThemeTrigger::Named,
                    },
                    nav + cta + named,
                ),
                (
                    BarFit {
                        nav: true,
                        cta: true,
                        trigger: ThemeTrigger::Bare,
                    },
                    nav + cta + bare,
                ),
                (
                    BarFit {
                        nav: true,
                        cta: false,
                        trigger: ThemeTrigger::Bare,
                    },
                    nav + bare,
                ),
            ];
            if let Some((fit, _)) = fuller.into_iter().find(|&(_, width)| width <= room) {
                return fit;
            }
        }
        // The compact bar keeps the bare label while the search stays
        // clear of the wordmark by more than the bar's action gap: the
        // actions may run into the gap after the wordmark (on a phone a
        // label is worth the tighter gap) but not up to the wordmark
        // itself. Short of that, the trigger keeps its icon alone.
        let clearance = row - logo - trigger_width(ThemeTrigger::Bare) - search;
        let trigger = if clearance > ACTIONS_GAP {
            ThemeTrigger::Bare
        } else {
            ThemeTrigger::Icon
        };
        BarFit {
            nav: false,
            cta: false,
            trigger,
        }
    }

    /// How wide the actions group runs, from the search button's left
    /// edge to the right edge of its last control. It must equal the
    /// extent of the left-to-right group `Gallery::app_bar` builds: the
    /// search button, the theme trigger and (where the bar has room) the
    /// call to action, each after the one before it by egui's item
    /// spacing and the bar's own `ACTIONS_GAP`. `app_bar` opens the
    /// group this far in from its right edge, so that the last control
    /// ends flush with it; a change to what the group holds or to the
    /// gaps between its controls changes this too.
    fn actions_width(&self, ui: &egui::Ui, theme: ThemeId) -> f32 {
        let between = ui.spacing().item_spacing.x + ACTIONS_GAP;
        let search = ui_kit::components::badge::ICON_BUTTON_SIZE;
        let through_trigger = search + between + trigger_button_width(ui, theme, &self.trigger);
        if self.cta {
            through_trigger + between + cta_button_width(ui, theme)
        } else {
            through_trigger
        }
    }
}

/// The width of the bar's theme trigger as `kind` reads: the default
/// button over its label with the chevron beside it, or the kit's icon
/// button, square at the search button's size.
fn trigger_button_width(ui: &egui::Ui, theme: ThemeId, kind: &ThemeTrigger) -> f32 {
    let labelled = |named: bool| {
        let label = theme_trigger_label(theme, named);
        button_width(ui, theme, ButtonSize::Default, &label, true)
    };
    match kind {
        ThemeTrigger::Named => labelled(true),
        ThemeTrigger::Bare => labelled(false),
        ThemeTrigger::Icon => ui_kit::components::badge::ICON_BUTTON_SIZE,
    }
}

/// The width of the bar's call to action, the small button over its
/// label.
fn cta_button_width(ui: &egui::Ui, theme: ThemeId) -> f32 {
    button_width(ui, theme, ButtonSize::Sm, copy::START_A_PROJECT, false)
}

/// The theme trigger's label: the menu's own, followed by the live
/// theme's name when the bar has room for it.
fn theme_trigger_label(theme: ThemeId, named: bool) -> String {
    if named {
        format!("{} · {}", copy::THEME_MENU, theme.name())
    } else {
        copy::THEME_MENU.to_owned()
    }
}

/// The width a kit `Button` takes for `label` at `size`, with or
/// without an icon beside it: the kit's own `min_width` over the label
/// as the button sets it (the semibold body face at the size's text
/// size, tracked open by the label's tracking).
fn button_width(
    ui: &egui::Ui,
    theme: ThemeId,
    size: ui_kit::components::button::ButtonSize,
    label: &str,
    icon: bool,
) -> f32 {
    use ui_kit::components::button::{min_width, LABEL_TRACKING_EM};
    use ui_kit::typography::{body_semibold_family, tracked_text};

    let font_id = egui::FontId {
        size: size.text_size(),
        family: body_semibold_family(theme),
    };
    let job = tracked_text(
        label,
        font_id,
        LABEL_TRACKING_EM,
        None,
        Color32::PLACEHOLDER,
        None,
    );
    let label_width = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job)).size().x;
    min_width(size, label_width, icon, false)
}

/// The width a kit `NavLink` takes for `label`: its label's line in the
/// link face, which is the whole of the link.
fn nav_link_width(ui: &egui::Ui, theme: ThemeId, label: &str) -> f32 {
    use ui_kit::components::top_app_bar::LINK_TEXT_SIZE;
    use ui_kit::typography::body_medium_family;

    let font_id = egui::FontId {
        size: LINK_TEXT_SIZE,
        family: body_medium_family(theme),
    };
    ui.ctx()
        .fonts_mut(|fonts| fonts.layout_delayed_color(label.to_owned(), font_id, f32::INFINITY))
        .size()
        .x
}

impl Gallery {
    /// The Brand panel (issue 10): the cover composition as the
    /// gallery's opening screen, then the Themes showcase and the
    /// Brand showcase, the brand book's three pages on one panel. The
    /// cover leads, so the panel needs no headline of its own.
    fn brand_panel(&mut self, ui: &mut egui::Ui) {
        Cover::new().ui(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        ThemesShowcase::new().ui(ui, self.theme, &self.flags);
        ui.add_space(spacing::SPACE_6);
        BrandShowcase::new().ui(ui, self.theme, &self.flags);
    }

    /// The window's chrome: the TopAppBar on Liquid Glass in a top
    /// panel, so it stays put while the page scrolls beneath (the
    /// web bar's sticky behaviour; the stylesheet's 96px
    /// scroll-padding-top reserve is the bar-and-clearance the panel
    /// provides here). The wordmark, the nav links, the search
    /// affordance, the theme menu and the one call to action, as much
    /// of it as the row has room for (`BarFit`).
    fn app_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, narrow: bool) {
        let theme = self.theme;
        let flags = self.flags;
        TopAppBar::new().ui(ui, theme, &flags, |ui| {
            ui.horizontal(|ui| {
                // The row is the bar's content height, so everything in
                // it centres on the bar's centre line. `horizontal` opens
                // the row at egui's 18px interact height, and an item
                // placed first and taller than that (the 24px wordmark,
                // the nav links, the call to action) is pinned to the
                // row's top, short of the 44px controls' centre line.
                ui.set_height(CONTENT_HEIGHT);
                // The bar's pane: the row grown by the padding of
                // `TopAppBar::ui`'s inner margin (`BAR_PAD_X` at the
                // sides, `BAR_PAD_Y` above and below), which this
                // inverts. It assumes `ui.max_rect()` is the frame's
                // content rect: the row spans its width, and
                // `set_height` above pins the row to its height. The
                // bar's popovers hang from the pane's bottom edge and
                // keep inside its left edge (`bar_popover`). Keep in
                // step with `TopAppBar::ui`'s inner margin.
                let bar = ui.max_rect().expand2(egui::vec2(BAR_PAD_X, BAR_PAD_Y));
                let fit = BarFit::measure(ui, theme, narrow);

                // The brand: the wordmark at its 24px chrome height,
                // in primary.
                ui.add(Logo::wordmark().height(LOGO_HEIGHT));
                ui.add_space(GROUP_GAP);

                // The primary nav: the panel links, the current one
                // carrying the indicator underline, where the row has
                // room for them (without them, the Tabs strip beneath
                // switches the same panels); the links return the
                // moment the window widens.
                if fit.nav {
                    for (index, label) in copy::TABS.iter().enumerate() {
                        if NavLink::new(label, self.tab == index)
                            .ui(ui, theme, &flags)
                            .clicked()
                        {
                            self.tab = index;
                        }
                        ui.add_space(LINK_GAP);
                    }
                }

                // The actions, flush right: the search affordance, the
                // theme menu, and the one CTA where the row has room
                // for it. With no CTA the theme trigger ends the row,
                // and its own padding sits inside the bar's, so its
                // icon would stand that much further in than the
                // wordmark does. The group is a child `Ui` that runs
                // past the row's right edge by the trigger's overhang
                // (a little less than that padding, to keep its focus
                // ring inside the bar): `with_layout` would stay inside
                // the row, and allocating the overhang would widen the
                // bar.
                //
                // The group is built left to right, in the order its
                // controls stand, because egui hands Tab focus along in
                // creation order, not layout order (built from the
                // right it ran CTA, theme, search). It still ends at
                // the right edge: the child opens `actions_width` in
                // from it.
                let row = ui.available_rect_before_wrap();
                let overhang = if fit.cta { 0.0 } else { fit.trigger.overhang() };
                let right = row.max.x + overhang;
                let mut actions = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            egui::pos2(right - fit.actions_width(ui, theme), row.min.y),
                            egui::pos2(right, row.max.y),
                        ))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                {
                    // The child, under the name the controls below use.
                    let ui = &mut actions;

                    // The search affordance: the 44px icon button
                    // opening the reused TextField in its popover. The
                    // open state is the Gallery's own, as the theme
                    // menu's is: the popover clears it when a tap lands
                    // outside it (or on Escape), and opening either of
                    // the bar's popovers closes the other.
                    let search = IconButton::new(copy::SEARCH_BUTTON_NAME, Icon::Search)
                        .ui(ui, theme, &flags);
                    if search.clicked() {
                        self.search_open = !self.search_open;
                        self.theme_open = false;
                    }
                    bar_popover(&search, bar, SEARCH_POPOVER_WIDTH, theme)
                        .open_bool(&mut self.search_open)
                        .show(|ui| {
                            TextField::new(copy::SEARCH_FIELD_LABEL, &mut self.search)
                                .placeholder(copy::SEARCH_PLACEHOLDER)
                                .leading_icon(Icon::Search)
                                .hint(copy::SEARCH_FIELD_HINT)
                                .ui(ui, theme, &flags);
                        });
                    // Keep in step with `BarFit::actions_width`.
                    ui.add_space(ACTIONS_GAP);

                    self.theme_menu(ui, ctx, fit.trigger, bar);

                    if fit.cta {
                        // Keep in step with `BarFit::actions_width`.
                        ui.add_space(ACTIONS_GAP);
                        Button::new(copy::START_A_PROJECT)
                            .small()
                            .emphasis(ButtonEmphasis::Tertiary)
                            .ui(ui, theme, &flags);
                    }
                }
                // The row claims its own rect, not the overhang.
                ui.advance_cursor_after_rect(row);
            });
        });
    }

    /// The theme menu: a kit button opening a kit popover, all six
    /// themes in a radio group and the reduced-motion and
    /// reduced-transparency toggles beneath them. The trigger reads as
    /// the bar's `BarFit` decides (`kind`): a default-size ghost button
    /// (the 44px touch floor, as quiet as the search icon beside it)
    /// naming the live theme where the bar has room for the name, or on
    /// the bare label where it has room for that, and on the narrowest
    /// phones the kit's 44px icon button with the tune icon, the search
    /// button's twin. The popover hangs under the bar `bar`
    /// ([`bar_popover`], as the search popover does), scrolls where the
    /// room under the bar is shorter than the menu ([`bar_popover_room`]),
    /// closes on a tap outside it, and opening it closes the search
    /// popover.
    fn theme_menu(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        kind: ThemeTrigger,
        bar: Rect,
    ) {
        let theme = self.theme;
        let flags = self.flags;
        let labelled = |ui: &mut egui::Ui, named: bool| {
            Button::new(&theme_trigger_label(theme, named))
                .emphasis(ButtonEmphasis::Ghost)
                .icon_trailing(Icon::ExpandMore)
                .ui(ui, theme, &flags)
                .response
        };
        let trigger = match kind {
            ThemeTrigger::Named => labelled(ui, true),
            ThemeTrigger::Bare => labelled(ui, false),
            // The kit's icon button, the search button's twin, named for
            // a screen reader by the menu's label alone.
            ThemeTrigger::Icon => {
                IconButton::new(copy::THEME_MENU, Icon::Tune).ui(ui, theme, &flags)
            }
        };
        if trigger.clicked() {
            self.theme_open = !self.theme_open;
            self.search_open = false;
        }
        // The open state is the Gallery's own: the popover clears it
        // when a tap lands outside it (or on Escape).
        let mut open = self.theme_open;
        let room = bar_popover_room(ctx, bar, theme);
        bar_popover(&trigger, bar, THEME_MENU_WIDTH, theme)
            .open_bool(&mut open)
            .show(|ui| {
                // Taller than the room under the bar (a landscape phone),
                // the menu scrolls inside it, a finger dragging it on
                // [`drag_scroll`]'s terms; where it fits, the area
                // shrinks to the content and changes nothing.
                //
                // The popup's `Ui` is as tall as the popup was last
                // frame (400pt, egui's default area height, while the
                // popup is first sized). Plain content overflows that,
                // and the popup measures the overflow; a scroll area takes
                // no more than its `Ui` offers, so left at that it would
                // hold the menu at 400pt less the frame, cutting its last
                // rows off in a window with room for them. The `Ui` is
                // given the room itself.
                ui.set_max_height(room);
                egui::ScrollArea::vertical()
                    .id_salt(THEME_MENU_SCROLL)
                    .max_height(room)
                    .scroll_source(ScrollSource {
                        drag: drag_scroll(),
                        ..ScrollSource::default()
                    })
                    .show(ui, |ui| self.theme_menu_content(ui, ctx));
            });
        self.theme_open = open;
    }

    /// The theme menu's popover: the six themes, then the two flags.
    /// The theme choice holds for the session (the Gallery's own
    /// field), and switching installs the theme's faces before applying
    /// it. The reduced-motion toggle shares its flag with the host,
    /// which sets it from the OS preference whenever that changes.
    fn theme_menu_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let theme = self.theme;
        let flags = self.flags;
        let names: Vec<&'static str> = ThemeId::ALL.iter().map(|id| id.name()).collect();
        let themes = radio_options(ThemeId::ALL, &names);
        // Every row is a touch target: the kit's rows are 20 and 26px
        // tall, so each is grown to the touch floor, and the theme rows
        // meet with no gap between them (the toggles keep the column's
        // own 3px item spacing). The radio group is indented so its
        // labels start where the toggles' do.
        let mut group = ui.available_rect_before_wrap();
        group.min.x += THEME_MENU_RADIO_INSET;
        let choice = ui
            .scope_builder(egui::UiBuilder::new().max_rect(group), |ui| {
                RadioGroup::new(copy::THEME_MENU)
                    .options(&themes)
                    .selected(Some(theme))
                    .min_row_height(button::TOUCH_TARGET_MIN)
                    .row_gap(0.0)
                    .ui(ui, theme, &flags)
            })
            .inner;
        if let Some(picked) = choice.committed {
            self.theme = picked;
            install(picked, ctx);
            apply_theme(picked, ctx);
        }

        ui.add_space(spacing::SPACE_5);
        if let Some(on) = Toggle::new(copy::REDUCED_MOTION)
            .checked(flags.reduced_motion)
            .min_row_height(button::TOUCH_TARGET_MIN)
            .ui(ui, theme, &flags)
            .toggled
        {
            self.flags.reduced_motion = on;
        }
        if let Some(on) = Toggle::new(copy::REDUCED_TRANSPARENCY)
            .checked(flags.reduced_transparency)
            .min_row_height(button::TOUCH_TARGET_MIN)
            .ui(ui, theme, &flags)
            .toggled
        {
            self.flags.reduced_transparency = on;
        }
    }

    /// The Foundations panel: the type specimen, the interaction
    /// states, the foundation specimens, the icons, the logo and the
    /// colour-role swatches.
    fn foundations_panel(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Label::new(specimen_job(
            copy::FOUNDATIONS_HEADLINE,
            TypeStyle::Headline2,
            self.theme,
            self.theme.colors().text_display(),
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_4);

        type_section(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        states_section(ui, self.theme, &self.flags);
        ui.add_space(spacing::SPACE_6);
        foundations_section(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        icons_section(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        logo_section(ui, self.theme);

        ui.add_space(spacing::SPACE_6);
        ui.label(format!(
            "{} · {}",
            copy::COLOUR_ROLES_LABEL,
            self.theme.name()
        ));
        ui.add_space(spacing::SPACE_2);
        let colors = self.theme.colors();
        egui::Grid::new("colour-roles")
            .num_columns(2)
            .spacing([spacing::SPACE_4, spacing::SPACE_2])
            .show(ui, |ui| {
                for role in ThemeColors::ROLES {
                    let color = colors
                        .role(role)
                        .expect("ROLES names all resolve to fields");
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    let radius = CornerRadius::same(radii::RADIUS_XS as u8);
                    ui.painter().rect_filled(rect, radius, color);
                    // The surfaces and glass tints sit close to the
                    // canvas and vanish without an edge. The edge is
                    // `outline`, the border role that holds 3:1 on the
                    // canvas in every theme (`outline_variant` is a
                    // 1.4:1 hairline, and translucent in the Teal
                    // themes), drawn outside so the swatch keeps its
                    // own colour.
                    ui.painter().rect_stroke(
                        rect,
                        radius,
                        egui::Stroke::new(1.0, colors.outline),
                        egui::StrokeKind::Outside,
                    );
                    ui.label(sentence_case(role));
                    ui.end_row();
                }
            });
    }

    /// The Components panel: the actions, the surfaces, the text
    /// inputs and the choice controls.
    fn components_panel(&mut self, ui: &mut egui::Ui) {
        actions_section(
            ui,
            self.theme,
            &self.flags,
            &mut self.filters,
            &mut self.saved_chip,
        );
        ui.add_space(spacing::SPACE_6);
        components_section(ui, self.theme, &self.flags);
        ui.add_space(spacing::SPACE_6);
        text_inputs_section(ui, self.theme, &self.flags, &mut self.form);
        ui.add_space(spacing::SPACE_6);
        choice_controls_section(
            ui,
            self.theme,
            &mut self.flags,
            &mut self.form,
            &mut self.choices,
        );
    }

    /// The Feedback panel: the banner demos at the panel top, the
    /// snackbar triggers below.
    fn feedback_panel(&mut self, ui: &mut egui::Ui) {
        feedback_section(
            ui,
            self.theme,
            &self.flags,
            &mut self.banners,
            &mut self.snackbar,
        );
    }
}

impl Gallery {
    /// The page under the tab strip: the active panel in the page's
    /// vertical scroll area. The wheel and the scroll bar scroll it
    /// everywhere, and egui's own drag-to-scroll on [`drag_scroll`]'s
    /// terms: on a phone from the very first touch, the page following
    /// the finger with the kinetic fling on release. The output's
    /// `content_size` is the panel's laid-out extent.
    ///
    /// When the content height changes, the frame is laid out again in
    /// the same frame if the panel ends up shorter than the offset the
    /// frame began at (a banner dismissed at the page's end). The gate
    /// compares the content height only: a restore lengthens the page and
    /// never fires it, and a change of the viewport that leaves the
    /// content height alone (a rotation, the keyboard) is not covered.
    ///
    /// egui places the content at the offset it stored and clamps that
    /// offset only after the content is placed, so the first frame with
    /// the shorter panel would be painted over-scrolled: the top of the
    /// panel under the tab strip and blank space below it. egui does
    /// follow that frame with one that paints the clamped page, but the
    /// Pixel was seen keeping the over-scrolled one on screen (finding
    /// 3-A9), so the frame is made right by itself: its second pass
    /// begins at the clamped offset, and only that pass reaches the
    /// screen. Only a changed height asks for it: a reader's own
    /// scrolling, the clamp at the page's end included, takes one pass.
    fn page(&mut self, ui: &mut egui::Ui) -> egui::scroll_area::ScrollAreaOutput<()> {
        let ctx = ui.ctx().clone();
        let salt = page_scroll_salt(self.tab);
        let page_id = ui.make_persistent_id(egui::IdSalt::new(salt));
        // A vertical drag that began on a sideways row arrives as the
        // offset change the row handed over last frame (`scrolling_row`).
        let forwarded = ctx.data_mut(|d| {
            std::mem::take(d.get_temp_mut_or_default::<f32>(egui::Id::new(PAGE_FORWARD)))
        });
        let mut area = egui::ScrollArea::vertical()
            .id_salt(salt)
            .scroll_source(ScrollSource {
                drag: drag_scroll(),
                ..ScrollSource::default()
            });
        // Loaded on every frame, not only when a row forwards a drag:
        // `began_at` below needs the stored offset each time.
        let stored =
            egui::scroll_area::State::load(&ctx, page_id).map_or(0.0, |state| state.offset.y);
        if forwarded != 0.0 {
            area = area.vertical_scroll_offset(stored + forwarded);
        }
        // The offset stored when the frame began. egui applies a drag's
        // and a fling's deltas inside `begin`, after the load above, so
        // the area may lay the content out a little away from it.
        let began_at = stored + forwarded;
        let page = area.show(ui, |ui| {
            match self.tab {
                0 => self.brand_panel(ui),
                1 => self.foundations_panel(ui),
                2 => self.components_panel(ui),
                _ => self.feedback_panel(ui),
            }
            ui.add_space(spacing::SPACE_6);
        });

        let height = page.content_size.y;
        let height_id = page_id.with(PAGE_HEIGHT);
        // The new height is stored on every pass, the one that asks for
        // the discard included, so the second pass reads `previous ==
        // height`, the gate stays shut and the frame discards once. The
        // offset test shuts it as well (the first pass stored the clamped
        // offset); a store skipped on the discard path would leave that
        // test as the only guard.
        let previous = ctx.data_mut(|d| {
            let previous = d.get_temp::<f32>(height_id);
            d.insert_temp(height_id, height);
            previous
        });
        let furthest = (height - page.inner_rect.height()).max(0.0);
        if previous.is_some_and(|previous| (previous - height).abs() > PAGE_PIXEL_SNAP)
            && began_at > furthest + PAGE_PIXEL_SNAP
        {
            ctx.request_discard("the page is shorter than the offset it was laid out at");
        }
        page
    }

    /// The window's chrome for one frame: the TopAppBar on Liquid Glass
    /// in a top panel, so it stays put while the page scrolls beneath
    /// (the web bar's sticky behaviour; the stylesheet's 96px
    /// scroll-padding-top reserve is the bar-and-clearance the panel
    /// provides here), the tabbed content panel, and the snackbar
    /// floating near the bottom.
    ///
    /// The root `Ui` eframe hands the app covers the whole viewport,
    /// safe areas included (egui's panels do not inset themselves —
    /// `rustnotes-core`'s scaffold does this for Rust Notes): the
    /// insets are read here and folded into the panels' outer margins,
    /// so the bar clears the status bar and Dynamic Island, the content
    /// clears the home indicator, and the snackbar rides above it. The
    /// viewport margins behind the insets are painted with the theme's
    /// canvas by `GalleryApp::ui`, so the fold is seamless.
    pub(crate) fn chrome(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        let insets = ctx.input(|i| i.safe_area_insets()).0;
        // The horizontal insets are mirrored: a cutout on one side of a
        // landscape phone would otherwise push the whole chrome to the
        // other side, so both gutters grow to the larger of the two.
        let side = insets.left.max(insets.right);
        // A short window (a phone in landscape) keeps most of its height
        // for the page: the gaps under the bar shrink to `SHORT_GAP`, and
        // the tab description below the strip goes.
        let short = root.available_height() < SHORT_WINDOW_HEIGHT;
        // `full` in a window of full height, `SHORT_GAP` in a short one.
        // The bar's clearance above is not one of these: it stays, and
        // only the gaps under the bar are compressed.
        let gap = |full: f32| if short { SHORT_GAP } else { full };
        let chrome_margin = |top: f32, bottom: f32| egui::Margin {
            left: (PAGE_GUTTER + side) as i8,
            right: (PAGE_GUTTER + side) as i8,
            top: (spacing::SPACE_3 + top) as i8,
            bottom: (gap(spacing::SPACE_3) + bottom) as i8,
        };
        // The content panel's margin: the gutter at each side as ever.
        // A short window gives up the 24px rhythm above the strip (for
        // `SHORT_GAP`) and below the page (no margin at all, as the page
        // ends with space of its own). It names `SHORT_GAP` rather than
        // using `gap`, which would repeat `content_margin`'s 24px here.
        let panel_margin = if short {
            egui::Margin {
                top: SHORT_GAP as i8,
                bottom: 0,
                ..content_margin()
            }
        } else {
            content_margin()
        };
        // On a phone-width window the bar carries the brand, the theme
        // menu and the search affordance only: the nav links and the
        // call to action would not fit beside the wordmark, and the
        // Tabs strip below the bar already switches every panel.
        let narrow = root.available_width() < NARROW_WIDTH;
        // The sticky bar, floating on the canvas with its clearance. The
        // bar is a rounded pane with a border of its own, so the
        // panel's separator line is off: a full-width straight line on
        // the panel's edge lay against that border and doubled it, and
        // ran on under the pane's rounded corners. The panel reserved
        // the line's room in its outer margin; the clearance keeps it,
        // so the page below the bar does not move. The room is the width
        // egui's `Panel::resolve_frame` reserves for the line (the
        // `noninteractive.bg_stroke` width), rounded the same way.
        let separator_room = root
            .style()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .width
            .round();
        egui::Panel::top("vynatix-app-bar")
            // Off, with `separator_room` standing in for the room it took.
            .show_separator_line(false)
            .frame(egui::Frame::default().outer_margin(chrome_margin(insets.top, separator_room)))
            .show(root, |ui| self.app_bar(ui, &ctx, narrow));

        egui::CentralPanel::default()
            .frame(egui::Frame::default().outer_margin(egui::Margin {
                left: side as i8,
                right: side as i8,
                top: 0,
                bottom: insets.bottom as i8,
            }))
            .show(root, |ui| {
                egui::Frame::default()
                    .inner_margin(panel_margin)
                    .show(ui, |ui| {
                        // The Tabs strip: the widget's own real demo, the
                        // gallery's panel switcher. It stays above the
                        // scroll, as navigation does, and where it
                        // outgrows a phone it drags as the page does.
                        let tabs = Tabs::new(copy::TABS)
                            .selected(self.tab)
                            .drag_to_scroll(drag_scroll())
                            .ui(ui, self.theme, &self.flags);
                        if let Some(moved_to) = tabs.changed {
                            self.tab = moved_to;
                        }
                        // The caption explains the panels and the arrow
                        // keys: on a phone it is noise above the content,
                        // and it names keys the phone does not have. A
                        // short window has no room for it either: three
                        // lines under the strip took a quarter of the
                        // Pixel's screen in landscape.
                        if !narrow && !short {
                            ui.add(egui::Label::new(specimen_job(
                                copy::TABS_NOTE,
                                TypeStyle::Caption,
                                self.theme,
                                self.theme.colors().text_muted(),
                                Some(ui.available_width()),
                            )));
                        }

                        ui.add_space(gap(spacing::SPACE_4));
                        let page = self.page(ui);
                        self.page_scroll_id = Some(page.id);
                        self.page_rect = Some(page.inner_rect);
                    });
            });

        // The snackbar host: the one visible message floats near the
        // window's bottom, centred, 24px above the edge. Its action
        // runs here, where the host knows what it enqueued.
        let action = self
            .snackbar
            .current()
            .and_then(|snackbar| snackbar.action.clone());
        if let Some(event) = self.snackbar.paint(&ctx, self.theme, &self.flags) {
            if event == SnackbarEvent::Action {
                match action.as_deref() {
                    Some(copy::SNACK_UNDO_ACTION) => {
                        self.snackbar
                            .show(Snackbar::success(copy::SNACK_RESULT_RESTORED));
                    }
                    Some(copy::SNACK_RETRY_ACTION) => {
                        self.snackbar
                            .show(Snackbar::success(copy::SNACK_RESULT_RUNNING));
                    }
                    Some(copy::SNACK_VIEW_ACTION) => {
                        self.snackbar
                            .show(Snackbar::info(copy::SNACK_RESULT_OPENING));
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_kit::components::metric_card::MIN_WIDTH;

    /// The narrowest phone the layout guard holds the panels to: 360pt,
    /// the common small Android width.
    const PHONE_WIDTH: f32 = 360.0;

    /// The Pixel emulator's window, 384pt: the width the usability pass
    /// saw the metric cards squeezed at.
    const PIXEL_WIDTH: f32 = 384.0;

    /// A desktop window, wide enough for every two-column layout.
    const DESKTOP_WIDTH: f32 = 1200.0;

    /// The panels the guard holds to the window's width: all four.
    const PANELS: [(usize, &str); 4] = [
        (0, "Brand"),
        (1, "Foundations"),
        (2, "Components"),
        (3, "Feedback"),
    ];

    /// Lay `tab`'s panel out on a window `width` wide inside the
    /// chrome's own page margins (the content panel's frame, the same
    /// [`content_margin`] `chrome` applies), and return how far the panel's content
    /// reaches past the width the page gives it: zero or less when it
    /// fits. The measure is the page area's `content_size`, the panel
    /// `Ui`'s own extent; the sideways specimen rows are wider inside
    /// their scroll areas by design, but their footprint in the panel
    /// is the panel's width.
    fn panel_overflow(tab: usize, width: f32) -> f32 {
        let ctx = egui::Context::default();
        ui_kit::icon::install(&ctx);
        for id in ThemeId::ALL {
            install(*id, &ctx);
        }
        apply_theme(ThemeId::Light, &ctx);
        let mut gallery = Gallery::new();
        gallery.tab = tab;
        let mut overflow = f32::NAN;
        // A few passes, so the sizing passes and the wrapped rows settle.
        for _ in 0..3 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::default())
                        .show(ui, |ui| {
                            egui::Frame::default()
                                .inner_margin(content_margin())
                                .show(ui, |ui| {
                                    let available = ui.available_width();
                                    let page = gallery.page(ui);
                                    overflow = page.content_size.x - available;
                                });
                        });
                },
            );
            out.drop_without_applying_deltas();
        }
        overflow
    }

    /// Every panel fits a 360pt phone: every row of fixed-width widgets
    /// wraps or scrolls, the card grid and the Brand tab's showcase
    /// cards stack, and nothing runs past the right edge to be clipped.
    #[test]
    fn phone_panels_fit_a_360pt_window() {
        for (tab, name) in PANELS {
            let overflow = panel_overflow(tab, PHONE_WIDTH);
            assert!(
                overflow <= 0.5,
                "{name} runs {overflow}pt past a {PHONE_WIDTH}pt window"
            );
        }
    }

    /// The same panels in their wide layouts (the card grid's two
    /// columns, the paired form columns, the showcase cards sharing
    /// rows) fit a desktop window.
    #[test]
    fn wide_panels_fit_a_desktop_window() {
        for (tab, name) in PANELS {
            let overflow = panel_overflow(tab, DESKTOP_WIDTH);
            assert!(
                overflow <= 0.5,
                "{name} runs {overflow}pt past a {DESKTOP_WIDTH}pt window"
            );
        }
    }

    /// Lay the Foundations STATES row out in `theme` on a desktop window
    /// and return, for each "Start a project" label it paints, the
    /// label's rect and the rect of the filled tile body it sits on (the
    /// last filled rect before the label that holds the label's centre;
    /// the focus ring's halo and outline are strokes with no fill).
    fn state_tile_labels(theme: ThemeId) -> Vec<(Rect, Rect)> {
        let ctx = egui::Context::default();
        install(theme, &ctx);
        apply_theme(theme, &ctx);
        let flags = UiKitFlags::default();
        let mut pairs = Vec::new();
        // The first pass binds the installed faces; the next let the
        // scroll row settle.
        for _ in 0..3 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(DESKTOP_WIDTH, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        states_section(ui, theme, &flags);
                    });
                },
            );
            let mut bodies: Vec<Rect> = Vec::new();
            pairs.clear();
            for clipped in &out.shapes {
                match &clipped.shape {
                    egui::Shape::Rect(rect) if rect.fill != Color32::TRANSPARENT => {
                        bodies.push(rect.rect);
                    }
                    egui::Shape::Text(text) if text.galley.text() == copy::START_A_PROJECT => {
                        let label = Rect::from_min_size(text.pos, text.galley.size());
                        let body = bodies
                            .iter()
                            .rev()
                            .find(|body| body.contains(label.center()))
                            .copied()
                            .expect("a label sits on a filled tile body");
                        pairs.push((label, body));
                    }
                    _ => {}
                }
            }
            out.drop_without_applying_deltas();
        }
        pairs
    }

    /// The STATES row's five tiles each hold the whole "Start a project"
    /// label inside their painted body, in every theme: the body takes
    /// its width from the label, so a wider face or a longer label
    /// widens the tile instead of spilling its ends onto the page.
    #[test]
    fn state_tile_labels_fit_inside_their_tiles() {
        for theme in ThemeId::ALL {
            let pairs = state_tile_labels(*theme);
            assert_eq!(
                pairs.len(),
                copy::STATE_CAPTIONS.len(),
                "{theme:?}: one label per state tile"
            );
            for (index, (label, body)) in pairs.iter().enumerate() {
                // The body is the tile's own fill: a focus ring around it
                // is taller than the tile, so one picked up here would
                // make the containment below too easy to meet.
                assert!(
                    body.height() <= spacing::SPACE_10 + 0.5,
                    "{theme:?}, tile {index}: the {}pt body is taller than the \
                     {}pt tile, so it is not the tile's fill",
                    body.height(),
                    spacing::SPACE_10,
                );
                assert!(
                    body.contains_rect(*label),
                    "{theme:?}, tile {index}: the {}pt label at x {}..{} runs outside \
                     its {}pt button at x {}..{}",
                    label.width(),
                    label.left(),
                    label.right(),
                    body.width(),
                    body.left(),
                    body.right(),
                );
            }
        }
    }

    /// Lay the Foundations STATES row out in `theme` on a desktop window
    /// and return the "Tab to me" button as painted: the rect of its
    /// outlined frame (the 1pt outline is drawn inside that rect), the
    /// outline's width, and the label's glyph box (the galley's rect).
    fn tab_to_me_pill(theme: ThemeId) -> (Rect, f32, Rect) {
        let ctx = egui::Context::default();
        install(theme, &ctx);
        apply_theme(theme, &ctx);
        let flags = UiKitFlags::default();
        let mut found = None;
        // The first pass binds the installed faces; the next let the
        // scroll row settle.
        for _ in 0..3 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(DESKTOP_WIDTH, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        states_section(ui, theme, &flags);
                    });
                },
            );
            let mut frames: Vec<(Rect, f32)> = Vec::new();
            found = None;
            for clipped in &out.shapes {
                match &clipped.shape {
                    egui::Shape::Rect(rect) if rect.stroke.width > 0.0 => {
                        frames.push((rect.rect, rect.stroke.width));
                    }
                    egui::Shape::Text(text) if text.galley.text() == copy::STATES_TAB_BUTTON => {
                        let label = Rect::from_min_size(text.pos, text.galley.size());
                        let (pill, outline) = frames
                            .iter()
                            .rev()
                            .find(|(frame, _)| frame.contains(label.center()))
                            .copied()
                            .expect("the label sits in an outlined frame");
                        found = Some((pill, outline, label));
                    }
                    _ => {}
                }
            }
            out.drop_without_applying_deltas();
        }
        found.expect("the states row paints the Tab-to-me label")
    }

    /// The least a compact label's pill stands tall: 24pt, room for the
    /// outline and the label with air around it.
    const MIN_PILL_HEIGHT: f32 = spacing::SPACE_6;

    /// The least air between the label's glyph box and the pill's
    /// outline, on each side.
    const MIN_PILL_AIR: f32 = spacing::SPACE_1;

    /// The "Tab to me" pill is at least 24pt tall, and its label keeps
    /// at least 4pt from the outline's inner edge on every side, in every
    /// theme: a pill at egui's 18pt default leaves the label touching it.
    #[test]
    fn the_tab_to_me_pill_has_room_for_its_label() {
        let mut short = Vec::new();
        for theme in ThemeId::ALL {
            let (pill, outline, label) = tab_to_me_pill(*theme);
            let size = format!(
                "pill {}x{}pt, outline {outline}pt, label {}x{}pt",
                pill.width(),
                pill.height(),
                label.width(),
                label.height(),
            );
            if pill.height() < MIN_PILL_HEIGHT - 0.01 {
                short.push(format!(
                    "{theme:?}: the pill is {}pt tall, under {MIN_PILL_HEIGHT}pt ({size})",
                    pill.height(),
                ));
            }
            let inner = pill.shrink(outline);
            let air = [
                ("left", label.left() - inner.left()),
                ("right", inner.right() - label.right()),
                ("top", label.top() - inner.top()),
                ("bottom", inner.bottom() - label.bottom()),
            ];
            for (side, gap) in air {
                if gap < MIN_PILL_AIR - 0.01 {
                    short.push(format!(
                        "{theme:?}: the {side} gap from the label to the outline is {gap}pt, \
                         under {MIN_PILL_AIR}pt ({size})"
                    ));
                }
            }
        }
        assert!(short.is_empty(), "{}", short.join("\n"));
    }

    /// Lay the Components tab's metric row out on a window `width` wide,
    /// inside the page's margins, and return the card bodies it painted
    /// (the rects filled with the card surface) with the right edge of
    /// the width the page gives the row.
    fn metric_cards(width: f32) -> (Vec<Rect>, f32) {
        let theme = ThemeId::Light;
        let ctx = egui::Context::default();
        install(theme, &ctx);
        apply_theme(theme, &ctx);
        let surface = theme.colors().surface_container_low;
        let mut bodies = Vec::new();
        let mut visible_right = f32::NAN;
        // The first pass binds the installed faces; the next let the
        // scroll row settle.
        for _ in 0..3 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::default())
                        .show(ui, |ui| {
                            egui::Frame::default()
                                .inner_margin(content_margin())
                                .show(ui, |ui| {
                                    visible_right = ui.max_rect().right();
                                    metrics_group(ui, theme);
                                });
                        });
                },
            );
            bodies.clear();
            for clipped in &out.shapes {
                filled_rects(&clipped.shape, surface, &mut bodies);
            }
            out.drop_without_applying_deltas();
        }
        (bodies, visible_right)
    }

    /// Collect the rects `shape` fills with `fill`. A frame with a shadow
    /// paints its shadow and its fill as one nested shape, so the walk
    /// descends into those.
    fn filled_rects(shape: &egui::Shape, fill: Color32, found: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Rect(rect) if rect.fill == fill => found.push(rect.rect),
            egui::Shape::Vec(nested) => {
                nested
                    .iter()
                    .for_each(|shape| filled_rects(shape, fill, found));
            }
            _ => {}
        }
    }

    /// The metric row keeps every card it paints at or above the kit's
    /// minimum on a phone window, and runs on past the window's edge to
    /// be scrolled to. A horizontal scroll caps its content at the
    /// visible width, so before the cards held a minimum the row gave
    /// each card what was left of that width: the second card came out
    /// narrow and the third one narrower, their labels broken a letter
    /// to a line. Card three sits wholly beyond the row's clip at this
    /// width, and a frame outside the clip paints nothing, so the test
    /// holds the cards that do paint.
    #[test]
    fn the_metric_row_scrolls_instead_of_squeezing_its_cards() {
        for width in [PHONE_WIDTH, PIXEL_WIDTH] {
            let (bodies, visible_right) = metric_cards(width);
            assert!(
                bodies.len() >= 2,
                "{width}pt: the row painted {} cards; at least the first two sit in view",
                bodies.len()
            );
            for (index, body) in bodies.iter().enumerate() {
                assert!(
                    body.width() >= MIN_WIDTH - 0.5,
                    "{width}pt: card {} is {}pt wide; the minimum is {MIN_WIDTH}pt",
                    index + 1,
                    body.width()
                );
            }
            assert!(
                bodies.iter().any(|body| body.right() > visible_right + 0.5),
                "{width}pt: every card fits inside the window, so the row squeezed them \
                 instead of scrolling"
            );
        }
    }

    /// Inside a wrapping row's tile the art and its caption keep the
    /// page's own item spacing: the row's wider spacing is for the gaps
    /// between tiles and between lines of tiles, not for the inside of
    /// one.
    #[test]
    fn a_tile_keeps_the_pages_spacing_between_its_art_and_caption() {
        let ctx = egui::Context::default();
        install(ThemeId::Light, &ctx);
        apply_theme(ThemeId::Light, &ctx);
        // The first pass binds the installed faces.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        let caption = "space-4 · 16";
        let mut page = Vec2::ZERO;
        let mut art = Rect::NOTHING;
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            page = ui.spacing().item_spacing;
            wrapping_row(ui, |ui, inherited| {
                wrapping_specimen(
                    ui,
                    inherited,
                    ThemeId::Light,
                    spacing::SPACE_24,
                    caption,
                    |ui| {
                        art = ui
                            .allocate_exact_size(
                                Vec2::new(spacing::SPACE_24, spacing::SPACE_2),
                                Sense::hover(),
                            )
                            .0;
                    },
                );
            });
        });
        let caption_top = out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == caption => Some(text.pos.y),
                _ => None,
            })
            .expect("the tile paints its caption");
        out.drop_without_applying_deltas();
        assert_eq!(
            caption_top - art.bottom(),
            page.y,
            "the caption sits the page's item spacing below the art"
        );
    }
}
