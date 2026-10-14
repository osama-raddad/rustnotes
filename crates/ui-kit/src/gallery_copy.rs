//! The gallery's copy: every string the gallery paints, held as
//! constants so the voice lint can scan it (issue 11).
//!
//! What counts as copy: the prose the gallery speaks — the chrome
//! labels, the section eyebrows and group labels, the explanatory
//! captions, and the demo content the components render (button
//! labels, form fields, banners, snackbars). What does not: strings
//! the code generates at run time from system data (type meta lines,
//! role names, icon names, theme names, counts), which carry no voice
//! of their own, and the identifier-shaped class names (`.card`,
//! `.glass`) that name CSS classes from the source previews verbatim.
//!
//! The brand page's Don't strings live in
//! [`crate::components::brand_showcase`] and are quoted
//! anti-patterns: examples of writing the brand forbids, held to the
//! punctuation checks but exempt from the vocabulary and sentence-case
//! disciplines they exist to break. The exemption travels with the
//! `Don't` label convention this crate already uses.
//!
//! The lint itself: [`voice_failures`] extends the seed
//! (`components::brand_showcase::copy_passes_voice_checks`) with the
//! vocabulary list, the sentence-case heuristic, the tighter
//! expected-glyph ranges and the ellipsis allowlist. Hosts may run it
//! over their own copy.

use crate::components::badge::BadgeTone;
use crate::typography::TypeStyle;

// ---- the chrome ---------------------------------------------------------------

/// The window and viewport title.
pub const APP_TITLE: &str = "Vynatix UI kit";

/// The gallery's four panels, in tab order: the brand book opens the
/// gallery, the cover its first screen.
pub const TABS: &[&str] = &["Brand", "Foundations", "Components", "Feedback"];

/// The caption under the tab strip: what each panel holds.
pub const TABS_NOTE: &str = "The tabs switch peer panels: Brand holds the cover and the brand book's themes and voice pages; Foundations the type, states, foundations, icons, logo and colour roles; Components the actions, surfaces, text inputs and choice controls; Feedback the banners and snackbars. Arrow keys move the selection while a tab holds focus.";

/// The bar's one call to action, and the Button preview's primary
/// specimen: the same words, the same job.
pub const START_A_PROJECT: &str = "Start a project";

/// The bar's theme menu label.
pub const THEME_MENU: &str = "Theme";

/// The theme menu's reduced-motion checkbox, and the Toggles preview's
/// own example use: one string, two surfaces.
pub const REDUCED_MOTION: &str = "Reduced motion";

/// The theme menu's reduced-transparency checkbox.
pub const REDUCED_TRANSPARENCY: &str = "Reduced transparency";

/// The bar's search affordance, its accessible name.
pub const SEARCH_BUTTON_NAME: &str = "Search the gallery";

/// The search popover's field label.
pub const SEARCH_FIELD_LABEL: &str = "Search";

/// The search placeholder: the bar's popover and the form's search
/// field share it.
pub const SEARCH_PLACEHOLDER: &str = "Case, essay or note";

/// The search popover's hint.
pub const SEARCH_FIELD_HINT: &str = "Everything in the gallery.";

// ---- the Foundations panel ------------------------------------------------------

/// The panel's headline.
pub const FOUNDATIONS_HEADLINE: &str = "Vynatix UI kit";

/// The type specimen's eyebrow.
pub const TYPE_EYEBROW: &str = "Materials · Type · 2026";

/// The type-specimen samples, the design system's own copy from
/// `tokens.json` `type.groups`, in specimen order.
pub const SPECIMEN: &[(TypeStyle, &str)] = &[
    (TypeStyle::Display, "Quiet is the new loud."),
    (
        TypeStyle::Headline1,
        "We rebuilt Volvo's truck radio in sixteen weeks.",
    ),
    (
        TypeStyle::Headline2,
        "A premium consultancy moves the way a turntable moves: deliberate, damped, confident.",
    ),
    (
        TypeStyle::Headline3,
        "Why does most consulting writing sound the same?",
    ),
    (
        TypeStyle::Headline4,
        "Engineering judgement, sold by the senior person doing the work.",
    ),
    (TypeStyle::Lede, "Vynatix is a Nordic IT consultancy in Gothenburg."),
    (
        TypeStyle::Body,
        "The discipline of the system is the luxury signal: not ornament, not saturation, not motion.",
    ),
    (
        TypeStyle::Editorial,
        "Premium isn't a price point. It's a discipline. The hundred small decisions a competitor would have skipped, made deliberately and visibly, day after day.",
    ),
    (TypeStyle::Eyebrow, "Field notes · Gothenburg · April 2026"),
    (
        TypeStyle::Mono,
        "--md-sys-color-primary: #1E3A2F;",
    ),
    (
        TypeStyle::Caption,
        "Photography by Jens Lindqvist. Volvo Trucks workshop, Lundby, Gothenburg. April 2026.",
    ),
];

/// The interaction states' eyebrow.
pub const STATES_EYEBROW: &str = "States · Hover, press, focus";

/// The state tiles' captions, specimen order.
pub const STATE_CAPTIONS: &[&str] = &[
    "Rest",
    "Hover · 8% on-colour overlay",
    "Pressed · 12% overlay · 98.5% scale · 100ms",
    "Focus · 2px ring · 2px offset · 4px halo · 120ms",
    "Disabled · content 38% · container 12%",
];

/// The focus specimen's button: the one plain control that really
/// takes focus via Tab.
pub const STATES_TAB_BUTTON: &str = "Tab to me";

/// The states section's closing caption.
pub const STATES_NOTE: &str =
    "Press Tab until the ring lands here. Focus is carried by the ring, not a colour change.";

/// The foundations specimens' eyebrow.
pub const FOUNDATIONS_EYEBROW: &str = "Foundations · Spacing, radii, elevation";

/// The spacing group's label.
pub const SPACING_LABEL: &str = "Spacing · the approved scale · 4px base";

/// The radii group's label.
pub const RADII_LABEL: &str = "Radii · all eight";

/// The elevation group's label; the active theme's name follows it.
pub const ELEVATION_LABEL: &str = "Elevation · L1 to L5";

/// The colour-role grid's label; the active theme's name follows it.
pub const COLOUR_ROLES_LABEL: &str = "Colour roles";

/// The icons section's eyebrow.
pub const ICONS_EYEBROW: &str = "Icons · Material Symbols Outlined · wght 300";

/// The icons section's caption: the subsetting rules.
pub const ICONS_NOTE: &str = "One library in every theme. Weight 300; grade -25 on the dark canvases so light glyphs don't bloom. The optical size sits at 24 in every subset. location_on has no glyph in font 0.47.5 and paints push_pin, the catalogue's pin.";

/// The logo section's eyebrow.
pub const LOGO_EYEBROW: &str = "Logo · The mark and the wordmark";

/// The canvas logo band's caption.
pub const LOGO_ON_CANVAS: &str = "On the canvas · primary";

/// The primary-fill logo band's caption.
pub const LOGO_ON_PRIMARY: &str = "On a primary fill · on-primary";

/// The inverse-band logo band's caption.
pub const LOGO_ON_INVERSE: &str = "On an inverse band · inverse-on-surface";

/// The accent-moment band's caption.
pub const LOGO_ACCENT_MOMENT: &str = "The accent moment · text-accent";

/// The logo section's closing caption: the size floors and clearance.
pub const LOGO_NOTE: &str = "Smallest on screen: the wordmark 96px wide, the mark 24px in product chrome and 96px in a brand placement. Clear space on every side: the height of the V.";

// ---- the actions section --------------------------------------------------------

/// The actions section's eyebrow.
pub const ACTIONS_EYEBROW: &str = "Components · Actions and labels";

/// The buttons group's label.
pub const BUTTONS_LABEL: &str = "Buttons · four emphases · radius-md · the body face at 600";

/// The Button preview's secondary specimen.
pub const BUTTON_READ_CASE_STUDY: &str = "Read the case study";

/// The Button preview's tertiary specimen, arrow trailing.
pub const BUTTON_HIRE_A_TEAM: &str = "Hire a team";

/// The Button preview's ghost specimen.
pub const BUTTON_CANCEL: &str = "Cancel";

/// The Button preview's disabled specimen.
pub const BUTTON_DISABLED: &str = "Disabled";

/// The Button preview's busy specimen. The gallery's one ellipsis: the
/// operational busy pattern the voice rules allow ("Sending…"), the
/// only entry in [`ELLIPSIS_ALLOWED`].
pub const BUTTON_BUSY: &str = "Sending…";

/// The buttons group's closing caption.
pub const BUTTONS_NOTE: &str = "Hover for the 8% on-colour overlay, hold for 12% and the 98.5% scale, Tab for the ring. Disabled drops to 38%. While an action runs, the label says what is happening.";

/// The sizes group's label.
pub const SIZES_LABEL: &str = "Sizes · 52, 44, 36 px";

/// The large button specimen.
pub const BUTTON_OPEN_DASHBOARD: &str = "Open the dashboard";

/// The default-size button specimen.
pub const BUTTON_SAVE_CHANGES: &str = "Save changes";

/// The small button specimen.
pub const BUTTON_FILTER_RESULTS: &str = "Filter results";

/// The sizes group's closing caption: the touch-target floor.
pub const SIZES_NOTE: &str = "Keep touch targets at 44px or more: the default and large sizes clear the floor, the 36px small size stays on pointer-precise surfaces.";

/// the icon-variants group's label.
pub const BUTTON_ICONS_LABEL: &str = "Icons · leading, trailing, icon-only";

/// The leading-icon button specimen.
pub const BUTTON_SEARCH_ARCHIVE: &str = "Search the archive";

/// The trailing-arrow button specimen.
pub const BUTTON_GET_PROPOSAL: &str = "Get the proposal";

/// The trailing-outward-arrow button specimen.
pub const BUTTON_VISIT_STUDIO: &str = "Visit the studio";

/// The search icon-only button's accessible name.
pub const BUTTON_SEARCH_EVERYTHING: &str = "Search everything";

/// the outward icon-only button's accessible name.
pub const BUTTON_OPEN_EXTERNAL: &str = "Open the external site";

/// The icon-variants group's closing caption.
pub const BUTTON_ICONS_NOTE: &str = "arrow_forward trails a call to action, arrow_outward a link that leaves the app. An icon-only button carries its action's name for assistive technology.";

/// The buttons group's last caption: the tertiary reservation.
pub const TERTIARY_NOTE: &str = "Tertiary is reserved: the one accent moment on a surface, never two of them on the same screen. Its label pair is the lowest in the system at 4.7 to 1, which the spec accepts.";

/// The chips group's label.
pub const CHIPS_LABEL: &str = "Chips · filter pills · radius-full";

/// The filter row's eyebrow.
pub const CHIPS_PRACTICE_AREA: &str = "Practice area";

/// The practice-area filter chips' labels.
pub const CHIP_FILTERS: &[&str] = &[
    "All",
    "Android Automotive",
    "Platform",
    "Design-engineering",
];

/// The featured accent chip.
pub const CHIP_FEATURED: &str = "Featured";

/// The chips group's closing caption.
pub const CHIPS_NOTE: &str = "A selected filter fills secondary-container and sets the check icon, so the state never rests on colour alone. The accent chip is the featured label.";

/// The dismissible chip's label.
pub const CHIP_CLIENT_WORK: &str = "Client work";

/// The button that brings the dismissed chip back: the row's empty
/// state, in words.
pub const CHIP_RESTORE: &str = "Restore the chip";

/// The dismissible demo's caption.
pub const CHIP_DISMISS_NOTE: &str =
    "The dismiss icon reports a typed event and the host drops the chip from its row.";

/// The badges group's label.
pub const BADGES_LABEL: &str = "Badges · states, kinds, counts";

/// The six tones' badge labels, the Badge preview's own words.
pub const BADGES: &[(&str, BadgeTone)] = &[
    ("Shipped", BadgeTone::Success),
    ("At risk", BadgeTone::Warning),
    ("In review", BadgeTone::Info),
    ("Blocked", BadgeTone::Error),
    ("Draft", BadgeTone::Neutral),
    ("Archived", BadgeTone::Neutral),
    ("Featured", BadgeTone::Accent),
];

/// The count-badge notifications button's accessible name.
pub const BADGE_NOTIFICATIONS: &str = "Notifications";

/// The count-badge mail button's accessible name.
pub const BADGE_MAIL: &str = "Mail";

/// The activity-dot messages button's accessible name.
pub const BADGE_MESSAGES: &str = "Messages";

/// The badges group's closing caption; the "3" is what
/// `count_text(3)` paints on the notifications button above it.
pub const BADGES_NOTE: &str = "One or two words on a tinted pill; the icon shape and the word carry the state. Counts read 3 through 99 and cap at 99+ beyond, pairing error with on-error so the count never competes with the accent.";

// ---- the surfaces section -------------------------------------------------------

/// The surfaces section's eyebrow.
pub const SURFACES_EYEBROW: &str = "Components · Surfaces · 2026";

/// The cards group's label.
pub const CARDS_LABEL: &str = "Cards · four variants · radius-lg · padding 24";

/// The default card specimen's class name, in mono.
pub const CARD_DEFAULT_CLASS: &str = ".card";

/// The default card specimen's description.
pub const CARD_DEFAULT_DESC: &str = "Elevated, the default: surface-container-low with shadow-l1.";

/// The outlined card specimen's class name.
pub const CARD_OUTLINED_CLASS: &str = ".card--outlined";

/// The outlined card specimen's description.
pub const CARD_OUTLINED_DESC: &str = "Surface with a 1px outline-variant border, no shadow.";

/// The filled card specimen's class name.
pub const CARD_FILLED_CLASS: &str = ".card--filled";

/// The filled card specimen's description.
pub const CARD_FILLED_DESC: &str = "Surface-container, no shadow.";

/// The interactive card specimen's class name.
pub const CARD_INTERACTIVE_CLASS: &str = ".card--interactive";

/// The interactive card specimen's description.
pub const CARD_INTERACTIVE_DESC: &str =
    "Lifts to surface-container and shadow-l2 on hover. Tab to it for the ring.";

/// The large-padding card specimen's class name.
pub const CARD_LARGE_CLASS: &str = ".card · large padding";

/// The large-padding card specimen's description.
pub const CARD_LARGE_DESC: &str =
    "The same elevated card at space-8, 32px, for pull-quotes and folios.";

/// The metrics group's label.
pub const METRICS_LABEL: &str = "Metrics · one number each, sourced";

/// The fleet metric's label.
pub const METRIC_FLEET_LABEL: &str = "Trucks running our code";

/// The fleet metric's figure.
pub const METRIC_FLEET_FIGURE: &str = "714k";

/// The fleet metric's source line.
pub const METRIC_FLEET_SOURCE: &str = "Volvo FH, FM, FMX. April 2026 fleet count.";

/// The tenure metric's label.
pub const METRIC_TENURE_LABEL: &str = "Average engagement length";

/// The tenure metric's figure.
pub const METRIC_TENURE_FIGURE: &str = "19 mo";

/// The tenure metric's source line.
pub const METRIC_TENURE_SOURCE: &str = "Across 27 engagements since 2022.";

/// The seniority metric's label.
pub const METRIC_SENIOR_LABEL: &str = "Senior people, percent";

/// The seniority metric's figure.
pub const METRIC_SENIOR_FIGURE: &str = "100";

/// The seniority metric's source line.
pub const METRIC_SENIOR_SOURCE: &str = "No juniors on client work. Ever.";

/// The seniority metric's unit suffix.
pub const METRIC_SENIOR_UNIT: &str = "%";

/// The editorial row's label.
pub const EDITORIAL_LABEL: &str = "Editorial · case study, field note, open role";

/// The case-study card's eyebrow.
pub const EDITORIAL_CASE_EYEBROW: &str = "Case study";

/// The case-study card's headline.
pub const EDITORIAL_CASE_HEADLINE: &str = "Rebuilding the truck radio in sixteen weeks.";

/// The case-study card's body.
pub const EDITORIAL_CASE_BODY: &str = "Three engineers, one designer, one product partner. Shipped to seven hundred thousand cabs by week thirty-one.";

/// The case-study card's action.
pub const EDITORIAL_CASE_ACTION: &str = "Read the case study";

/// The field-note card's eyebrow.
pub const EDITORIAL_FIELD_EYEBROW: &str = "Field notes";

/// The field-note card's headline.
pub const EDITORIAL_FIELD_HEADLINE: &str = "Premium isn't a price point. It's a discipline.";

/// The field-note card's body.
pub const EDITORIAL_FIELD_BODY: &str = "Why every Vynatix interface starts from a sixty-thirty-ten ratio, even when the brief asks for something louder.";

/// The field-note card's action.
pub const EDITORIAL_FIELD_ACTION: &str = "Read the field note";

/// The open-role card's eyebrow.
pub const EDITORIAL_ROLE_EYEBROW: &str = "Open role";

/// The open-role card's headline.
pub const EDITORIAL_ROLE_HEADLINE: &str = "Senior platform engineer, Gothenburg.";

/// The open-role card's body.
pub const EDITORIAL_ROLE_BODY: &str =
    "Android Automotive, Kotlin, OTA pipelines. Permanent. Pay band published. No negotiation theatre.";

/// The open-role card's action.
pub const EDITORIAL_ROLE_ACTION: &str = "Read the role";

/// The glass group's label.
pub const GLASS_LABEL: &str = "Liquid glass · five thicknesses · 1px specular edge · shadow-l2";

/// The tints row's label.
pub const GLASS_TINTS_LABEL: &str =
    "Tints and the fallback · regular thickness · over the page canvas";

/// The surface tint's class name.
pub const GLASS_SURFACE_CLASS: &str = ".glass";

/// The primary tint's class name.
pub const GLASS_PRIMARY_CLASS: &str = ".glass--primary";

/// The accent tint's class name.
pub const GLASS_ACCENT_CLASS: &str = ".glass--accent";

/// The reduced-transparency tile's name: the fallback pane.
pub const GLASS_FALLBACK_NAME: &str = "Reduced transparency";

/// The live chrome pane's label, in the recipe's colour.
pub const GLASS_CHROME_LABEL: &str = "A pane as chrome, its labels in the recipe's colour";

/// The glass group's closing caption: the usage rules.
pub const GLASS_NOTE: &str = "Chrome and overlays only: top app bars, sheets, popovers, modal headers. Never body content, and never two panes of the same thickness stacked. Under reduced transparency every pane turns solid, as the fourth tile shows. egui composites no backdrop blur: the tint and its opacity carry the glass, and the 1px specular edge marks the pane.";

// ---- the text inputs section ----------------------------------------------------

/// The text inputs section's eyebrow.
pub const INPUTS_EYEBROW: &str = "Components · Text inputs";

/// The fields group's label.
pub const FIELDS_LABEL: &str = "Fields · label, control, hint · radius-md · 12 by 14 padding";

/// The project-name field's label.
pub const FIELD_NAME_LABEL: &str = "Project name";

/// The project-name field's placeholder.
pub const FIELD_NAME_PLACEHOLDER: &str = "In-cab radio, phase two";

/// The project-name field's hint.
pub const FIELD_NAME_HINT: &str = "As it appears in the proposal.";

/// The work-email field's label; the asterisk marks it required.
pub const FIELD_EMAIL_LABEL: &str = "Work email *";

/// The work-email field's placeholder, the format hint.
pub const FIELD_EMAIL_PLACEHOLDER: &str = "name@company.se";

/// The work-email field's hint.
pub const FIELD_EMAIL_HINT: &str = "Required. We will never sell or share your email.";

/// The form's search field's label.
pub const FIELD_SEARCH_LABEL: &str = "Search the case studies";

/// The reference-number field's label.
pub const FIELD_REFERENCE_LABEL: &str = "Reference number";

/// The locked fields' shared hint: the reference number and the
/// invoice currency are both set by the contract.
pub const SET_BY_CONTRACT: &str = "Set by your contract.";

/// The brief textarea's label.
pub const FIELD_BRIEF_LABEL: &str = "What are you trying to ship?";

/// The brief textarea's placeholder.
pub const FIELD_BRIEF_PLACEHOLDER: &str = "Two sentences is plenty.";

/// The email error, per the voice guidelines' four parts: what is
/// wrong, why it matters, what to do next, where to get help.
pub const EMAIL_ERROR: &str = "That address is missing its @ sign or its domain, so the proposal would not arrive. Type the full address, like name@company.se. If you are unsure which one to use, ask your account manager.";

/// The fields group's closing caption.
pub const FIELDS_NOTE: &str = "The label stays visible when the field fills, and the placeholder is a format hint, never a substitute. The hint carries the rule; the error replaces it, says what is wrong and how to fix it, and the icon keeps the state off colour alone. The email checks when you leave the field, not while you type. Tab through the fields to see each ring.";

/// The form's prefilled reference number, locked by the contract.
pub const REFERENCE_DEFAULT: &str = "A-2026-114";

/// The form's prefilled brief.
pub const BRIEF_DEFAULT: &str = "Replacing the in-cab radio firmware before the autumn launch: Android Automotive, three-screen layout, OTA pipeline.";

/// The selects group's label.
pub const SELECTS_LABEL: &str = "Select · the same box with a chevron · popover list";

/// The region options' labels, the select's and the radio group's
/// shared domain.
pub const REGIONS: &[&str] = &["Nordics", "Europe", "Global"];

/// The delivery offices' labels: nine of them, so the list scrolls
/// inside the eight rows the kit caps it at.
pub const OFFICES: &[&str] = &[
    "Gothenburg",
    "Stockholm",
    "Copenhagen",
    "Oslo",
    "Helsinki",
    "Berlin",
    "Amsterdam",
    "London",
    "Edinburgh",
];

/// The team sizes' labels, the Select preview's own options.
pub const TEAM_SIZES: &[&str] = &["1 to 2 people", "3 to 5 people", "6 or more"];

/// The invoice currency's label: one option, locked.
pub const CURRENCIES: &[&str] = &["SEK"];

/// The region select's label.
pub const SELECT_REGION_LABEL: &str = "Region";

/// The region select's and radio group's shared hint.
pub const SELECT_REGION_HINT: &str = "Where the team will work from.";

/// The delivery-office select's label.
pub const SELECT_OFFICE_LABEL: &str = "Delivery office";

/// The delivery-office select's hint.
pub const SELECT_OFFICE_HINT: &str = "Nine offices, so the list scrolls.";

/// The team-size select's label; the asterisk marks it required.
pub const SELECT_TEAM_LABEL: &str = "Team size *";

/// The team-size select's placeholder.
pub const SELECT_TEAM_PLACEHOLDER: &str = "Choose a team size";

/// The team-size select's hint.
pub const SELECT_TEAM_HINT: &str = "Required. We staff one team per project.";

/// The invoice-currency select's label.
pub const SELECT_CURRENCY_LABEL: &str = "Invoice currency";

/// The selects group's closing caption.
pub const SELECTS_NOTE: &str = "The select commits on a click or Enter, and Escape or a click outside closes it without changing the choice. The check marks the selected option, never colour alone. The arrow keys move the hover, wrapping; a list longer than eight options scrolls inside eight rows.";

// ---- the choice controls section --------------------------------------------------

/// The choice controls section's eyebrow.
pub const CHOICE_EYEBROW: &str = "Components · Choice controls";

/// The checkboxes group's label.
pub const CHECKBOXES_LABEL: &str = "Checkboxes · one yes or no, or any number from a group";

/// The practice-areas group's label.
pub const CHECK_AREAS_LABEL: &str = "Practice areas";

/// The practice-areas parent's label.
pub const CHECK_PARENT_LABEL: &str = "All practice areas";

/// The practice-areas group's hint.
pub const CHECK_AREAS_HINT: &str = "Choose all that apply.";

/// The nested practice-area children's labels.
pub const PRACTICE_AREAS: &[&str] = &["Android Automotive", "Platform", "Design-engineering"];

/// The notifications group's label.
pub const CHECK_NOTIFICATIONS_LABEL: &str = "Notifications";

/// The weekly-digest checkbox's label.
pub const CHECK_DIGEST_LABEL: &str = "Weekly digest";

/// The weekly-digest checkbox's hint.
pub const CHECK_DIGEST_HINT: &str = "Every Monday at 08:00.";

/// The blocked-builds checkbox's label.
pub const CHECK_BUILDS_LABEL: &str = "Blocked builds";

/// The blocked-builds checkbox's hint.
pub const CHECK_BUILDS_HINT: &str = "When a pipeline has failed for over an hour.";

/// The disabled text-messages checkbox's label.
pub const CHECK_TEXTS_LABEL: &str = "Text messages";

/// The disabled text-messages checkbox's hint.
pub const CHECK_TEXTS_HINT: &str = "Add a mobile number to your profile first.";

/// The radio groups' label.
pub const RADIOS_LABEL: &str = "Radio group · exactly one of a few, side by side";

/// The invalid start group's label: the group carries no state, the
/// error is the demo.
pub const RADIO_START_LABEL: &str = "Start";

/// The start options' labels.
pub const START_OPTIONS: &[&str] = &["This quarter", "Next quarter", "Not decided yet"];

/// The invalid start group's error.
pub const RADIO_START_ERROR: &str = "Choose when the team should start.";

/// The toggles group's label.
pub const TOGGLES_LABEL: &str = "Toggles · settings that take effect at once";

/// The local toggle's label.
pub const TOGGLE_FILTERS_LABEL: &str = "Remember my filters";

/// The disabled sync toggle's label.
pub const TOGGLE_SYNC_LABEL: &str = "Sync over mobile data";

/// The disabled sync toggle's hint.
pub const TOGGLE_SYNC_HINT: &str = "Ask your account lead to switch this on.";

/// The disabled timesheets toggle's label.
pub const TOGGLE_TIMESHEETS_LABEL: &str = "Autofill timesheets";

/// The disabled timesheets toggle's hint.
pub const TOGGLE_TIMESHEETS_HINT: &str = "Needs an approved project code first.";

/// The choice controls section's closing caption.
pub const CHOICE_NOTE: &str = "A checkbox turns an option on or off: one yes or no, or any number from a group, the parent marking partly-chosen children. A radio group picks exactly one of a few that belong side by side. A toggle switches a setting that takes effect at once: the knob slides 200ms on the house curve and snaps under reduced motion. Box and circle are 18px behind a 2px outline, the label rides beside in the body face, and focus draws the shared ring.";

// ---- the feedback section --------------------------------------------------------

/// The feedback section's eyebrow.
pub const FEEDBACK_EYEBROW: &str = "Components · Feedback";

/// The banners group's label.
pub const BANNERS_LABEL: &str = "Banners · info, success, warning, error · radius-md";

/// The info banner's title.
pub const BANNER_INFO_TITLE: &str = "Saved automatically.";

/// The info banner's message.
pub const BANNER_INFO_MESSAGE: &str = "We kept your last six edits: refresh to see them.";

/// The success banner's title.
pub const BANNER_SUCCESS_TITLE: &str = "Timesheet approved.";

/// The success banner's message.
pub const BANNER_SUCCESS_MESSAGE: &str = "Week 41 is locked and ready for invoicing.";

/// The warning banner's title.
pub const BANNER_WARNING_TITLE: &str = "Your assignment ends in 14 days.";

/// The warning banner's message.
pub const BANNER_WARNING_MESSAGE: &str = "Talk to your account lead about an extension.";

/// The error banner's title.
pub const BANNER_ERROR_TITLE: &str = "We couldn't save your changes.";

/// The error banner's message, carrying the four-part voice.
pub const BANNER_ERROR_MESSAGE: &str = "The connection dropped mid-save, so your edits exist only on this machine. Try again from the toolbar; if it keeps failing, ask your account lead to check the sync log.";

/// The button that brings the dismissed banners back: the section's
/// empty state, in words.
pub const BANNERS_RESTORE: &str = "Restore the banners";

/// The banners group's closing caption.
pub const BANNERS_NOTE: &str = "A banner carries important information at the top of a page or section: the icon and the words hold the state, never the colour alone. Info and success dismiss; warnings and errors stay until the state resolves, which is why theirs carry no close button. Error copy leads with what is wrong, then why it matters, what to do next and where to get help.";

/// The snackbars group's label.
pub const SNACKBARS_LABEL: &str =
    "Snackbars · inverse surface · one at a time · 6s, 10s with an action";

/// The save snackbar's trigger.
pub const SNACK_SAVE_TRIGGER: &str = "Save the brief";

/// The copy snackbar's trigger.
pub const SNACK_COPY_TRIGGER: &str = "Copy the link";

/// The submit snackbar's trigger.
pub const SNACK_SUBMIT_TRIGGER: &str = "Submit the timesheet";

/// The archive snackbar's trigger.
pub const SNACK_ARCHIVE_TRIGGER: &str = "Archive the workspace";

/// The sync snackbar's trigger.
pub const SNACK_SYNC_TRIGGER: &str = "Sync now";

/// The save snackbar's message.
pub const SNACK_SAVED: &str = "Saved.";

/// The copy snackbar's message.
pub const SNACK_LINK_COPIED: &str = "Link copied.";

/// The submit snackbar's message.
pub const SNACK_SUBMITTED: &str = "Timesheet submitted for week 41.";

/// The submit snackbar's action.
pub const SNACK_VIEW_ACTION: &str = "View";

/// The archive snackbar's title.
pub const SNACK_ARCHIVED_TITLE: &str = "Workspace archived";

/// The archive snackbar's line.
pub const SNACK_ARCHIVED_LINE: &str = "Read-only access for 30 days, then permanent removal.";

/// The archive snackbar's action.
pub const SNACK_UNDO_ACTION: &str = "Undo";

/// The sync snackbar's title.
pub const SNACK_SYNC_FAILED_TITLE: &str = "Sync failed";

/// The sync snackbar's line.
pub const SNACK_SYNC_FAILED_LINE: &str = "Upstream returned 503. Retrying automatically.";

/// The sync snackbar's action.
pub const SNACK_RETRY_ACTION: &str = "Retry now";

/// What the Undo action reports.
pub const SNACK_RESULT_RESTORED: &str = "Workspace restored.";

/// What the Retry now action reports.
pub const SNACK_RESULT_RUNNING: &str = "Sync running.";

/// What the View action reports.
pub const SNACK_RESULT_OPENING: &str = "Opening week 41.";

/// The snackbars group's closing caption.
pub const SNACKBARS_NOTE: &str = "One short sentence, outcome first, on the inverse surface at the bottom of the window. Info and success leave after about six seconds, ten with an action; the timer pauses while the pointer or the focus is inside; errors stay until dismissed or resolved. A second message replaces the first: one at a time. Reduced motion shortens the entrance and never the timing.";

// ---- the voice lint -------------------------------------------------------------

/// The words the brand voice forbids, lowercase; a hit is a failure
/// wherever it appears. "revolutionize" rides along with the issue's
/// "revolutionise" so the American spelling cannot slip through.
pub const FORBIDDEN_VOCABULARY: &[&str] = &[
    "leverage",
    "empower",
    "unlock",
    "world-class",
    "best-in-class",
    "cutting-edge",
    "next-generation",
    "disruptive",
    "transformative",
    "supercharge",
    "revolutionise",
    "revolutionize",
    "game-changing",
];

/// The copy that may carry an ellipsis: the operational busy pattern
/// alone ("Sending…"). The voice rules keep the ellipsis out of
/// prose; a control mid-action is the one place it works.
const ELLIPSIS_ALLOWED: &[&str] = &[BUTTON_BUSY];

/// The glyphs gallery copy may use: ASCII text and punctuation, the
/// Latin-1 supplement (the middle dot ·, the accented é in "cliché"),
/// the curly single quotes the copy's apostrophes set, and the
/// ellipsis, which its own rule gates. Everything else — every emoji
/// block, the dingbats, the arrows and stars, the variation selectors
/// and joiners, and any script beyond Latin — fails.
fn is_expected_text_char(c: char) -> bool {
    matches!(c as u32,
        0x20..=0x7E         // ASCII: the text, the digits, the punctuation
        | 0xA0..=0xFF       // Latin-1: · the separator, é in cliché
        | 0x2018..=0x2019   // the curly quotes: Don't, isn't
        | 0x2026             // the ellipsis, gated by ELLIPSIS_ALLOWED
    )
}

/// Whether `text` reads as fully title-cased over more than three
/// words: every word (whitespace tokens that carry a letter or digit,
/// separator dots dropped) begins on an uppercase letter or a digit.
/// The sentence-case heuristic the voice enforces on headings; proper
/// nouns and short labels pass because they never run past three
/// words.
fn is_fully_title_cased(text: &str) -> bool {
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .collect();
    words.len() > 3
        && words.iter().all(|word| {
            word.chars()
                .find(|first| first.is_alphanumeric())
                .is_some_and(|first| first.is_uppercase() || first.is_numeric())
        })
}

/// Run the gallery voice lint over one string, returning every failure
/// it finds (empty means the copy passes).
///
/// The checks, the brand book's own: no em dashes (the hyphen and the
/// colon carry us), no ellipses outside the operational busy patterns,
/// no exclamation marks, no glyphs beyond the expected text ranges
/// (emoji included), none of [`FORBIDDEN_VOCABULARY`], and no string
/// fully title-cased past three words (sentence case for headings).
///
/// `not_our_prose` marks a string the kit paints but does not speak
/// as its own voice: the brand page's quoted anti-patterns and the
/// theme cards' ratio legends (paint names and shares, every word a
/// proper noun or a digit). The vocabulary and sentence-case checks
/// step aside for those — the Don'ts exist to break them, the legends
/// are names — while the punctuation checks always run.
pub fn voice_failures(text: &str, not_our_prose: bool) -> Vec<String> {
    let mut failures = Vec::new();
    if let Some(glyph) = text.chars().find(|c| !is_expected_text_char(*c)) {
        failures.push(format!(
            "unexpected glyph {glyph:?} (emoji or out-of-range)"
        ));
    }
    if text.contains('\u{2014}') {
        failures.push("em dash (write it with a colon or a hyphen)".to_owned());
    }
    if text.contains('\u{2026}') && !ELLIPSIS_ALLOWED.contains(&text) {
        failures.push("ellipsis outside the operational busy patterns".to_owned());
    }
    if text.contains('!') {
        failures.push("exclamation mark".to_owned());
    }
    if !not_our_prose {
        let lower = text.to_lowercase();
        for word in FORBIDDEN_VOCABULARY {
            if lower.contains(word) {
                failures.push(format!("forbidden vocabulary: {word}"));
            }
        }
        if is_fully_title_cased(text) {
            failures.push("fully title-cased past three words (sentence case)".to_owned());
        }
    }
    failures
}

/// Whether `text` passes the lint; see [`voice_failures`].
pub fn passes_voice_lint(text: &str, not_our_prose: bool) -> bool {
    voice_failures(text, not_our_prose).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every copy constant this module holds, the registry the lint
    /// scans. A new constant joins this list; the lint then holds it
    /// to the voice the way the brand book states it.
    fn all_copy() -> Vec<&'static str> {
        let mut texts = vec![
            APP_TITLE,
            TABS_NOTE,
            START_A_PROJECT,
            THEME_MENU,
            REDUCED_MOTION,
            REDUCED_TRANSPARENCY,
            SEARCH_BUTTON_NAME,
            SEARCH_FIELD_LABEL,
            SEARCH_PLACEHOLDER,
            SEARCH_FIELD_HINT,
            FOUNDATIONS_HEADLINE,
            TYPE_EYEBROW,
            STATES_EYEBROW,
            STATES_TAB_BUTTON,
            STATES_NOTE,
            FOUNDATIONS_EYEBROW,
            SPACING_LABEL,
            RADII_LABEL,
            ELEVATION_LABEL,
            COLOUR_ROLES_LABEL,
            ICONS_EYEBROW,
            ICONS_NOTE,
            LOGO_EYEBROW,
            LOGO_ON_CANVAS,
            LOGO_ON_PRIMARY,
            LOGO_ON_INVERSE,
            LOGO_ACCENT_MOMENT,
            LOGO_NOTE,
            ACTIONS_EYEBROW,
            BUTTONS_LABEL,
            START_A_PROJECT,
            BUTTON_READ_CASE_STUDY,
            BUTTON_HIRE_A_TEAM,
            BUTTON_CANCEL,
            BUTTON_DISABLED,
            BUTTON_BUSY,
            BUTTONS_NOTE,
            SIZES_LABEL,
            BUTTON_OPEN_DASHBOARD,
            BUTTON_SAVE_CHANGES,
            BUTTON_FILTER_RESULTS,
            SIZES_NOTE,
            BUTTON_ICONS_LABEL,
            BUTTON_SEARCH_ARCHIVE,
            BUTTON_GET_PROPOSAL,
            BUTTON_VISIT_STUDIO,
            BUTTON_SEARCH_EVERYTHING,
            BUTTON_OPEN_EXTERNAL,
            BUTTON_ICONS_NOTE,
            TERTIARY_NOTE,
            CHIPS_LABEL,
            CHIPS_PRACTICE_AREA,
            CHIP_FEATURED,
            CHIPS_NOTE,
            CHIP_CLIENT_WORK,
            CHIP_RESTORE,
            CHIP_DISMISS_NOTE,
            BADGES_LABEL,
            BADGE_NOTIFICATIONS,
            BADGE_MAIL,
            BADGE_MESSAGES,
            BADGES_NOTE,
            SURFACES_EYEBROW,
            CARDS_LABEL,
            CARD_DEFAULT_CLASS,
            CARD_DEFAULT_DESC,
            CARD_OUTLINED_CLASS,
            CARD_OUTLINED_DESC,
            CARD_FILLED_CLASS,
            CARD_FILLED_DESC,
            CARD_INTERACTIVE_CLASS,
            CARD_INTERACTIVE_DESC,
            CARD_LARGE_CLASS,
            CARD_LARGE_DESC,
            METRICS_LABEL,
            METRIC_FLEET_LABEL,
            METRIC_FLEET_FIGURE,
            METRIC_FLEET_SOURCE,
            METRIC_TENURE_LABEL,
            METRIC_TENURE_FIGURE,
            METRIC_TENURE_SOURCE,
            METRIC_SENIOR_LABEL,
            METRIC_SENIOR_FIGURE,
            METRIC_SENIOR_SOURCE,
            METRIC_SENIOR_UNIT,
            EDITORIAL_LABEL,
            EDITORIAL_CASE_EYEBROW,
            EDITORIAL_CASE_HEADLINE,
            EDITORIAL_CASE_BODY,
            EDITORIAL_CASE_ACTION,
            EDITORIAL_FIELD_EYEBROW,
            EDITORIAL_FIELD_HEADLINE,
            EDITORIAL_FIELD_BODY,
            EDITORIAL_FIELD_ACTION,
            EDITORIAL_ROLE_EYEBROW,
            EDITORIAL_ROLE_HEADLINE,
            EDITORIAL_ROLE_BODY,
            EDITORIAL_ROLE_ACTION,
            GLASS_LABEL,
            GLASS_TINTS_LABEL,
            GLASS_SURFACE_CLASS,
            GLASS_PRIMARY_CLASS,
            GLASS_ACCENT_CLASS,
            GLASS_FALLBACK_NAME,
            GLASS_CHROME_LABEL,
            GLASS_NOTE,
            INPUTS_EYEBROW,
            FIELDS_LABEL,
            FIELD_NAME_LABEL,
            FIELD_NAME_PLACEHOLDER,
            FIELD_NAME_HINT,
            FIELD_EMAIL_LABEL,
            FIELD_EMAIL_PLACEHOLDER,
            FIELD_EMAIL_HINT,
            FIELD_SEARCH_LABEL,
            FIELD_REFERENCE_LABEL,
            SET_BY_CONTRACT,
            FIELD_BRIEF_LABEL,
            FIELD_BRIEF_PLACEHOLDER,
            EMAIL_ERROR,
            FIELDS_NOTE,
            REFERENCE_DEFAULT,
            BRIEF_DEFAULT,
            SELECTS_LABEL,
            SELECT_REGION_LABEL,
            SELECT_REGION_HINT,
            SELECT_OFFICE_LABEL,
            SELECT_OFFICE_HINT,
            SELECT_TEAM_LABEL,
            SELECT_TEAM_PLACEHOLDER,
            SELECT_TEAM_HINT,
            SELECT_CURRENCY_LABEL,
            SELECTS_NOTE,
            CHOICE_EYEBROW,
            CHECKBOXES_LABEL,
            CHECK_PARENT_LABEL,
            CHECK_AREAS_LABEL,
            CHECK_AREAS_HINT,
            CHECK_NOTIFICATIONS_LABEL,
            CHECK_DIGEST_LABEL,
            CHECK_DIGEST_HINT,
            CHECK_BUILDS_LABEL,
            CHECK_BUILDS_HINT,
            CHECK_TEXTS_LABEL,
            CHECK_TEXTS_HINT,
            RADIOS_LABEL,
            RADIO_START_LABEL,
            RADIO_START_ERROR,
            TOGGLES_LABEL,
            TOGGLE_FILTERS_LABEL,
            TOGGLE_SYNC_LABEL,
            TOGGLE_SYNC_HINT,
            TOGGLE_TIMESHEETS_LABEL,
            TOGGLE_TIMESHEETS_HINT,
            CHOICE_NOTE,
            FEEDBACK_EYEBROW,
            BANNERS_LABEL,
            BANNER_INFO_TITLE,
            BANNER_INFO_MESSAGE,
            BANNER_SUCCESS_TITLE,
            BANNER_SUCCESS_MESSAGE,
            BANNER_WARNING_TITLE,
            BANNER_WARNING_MESSAGE,
            BANNER_ERROR_TITLE,
            BANNER_ERROR_MESSAGE,
            BANNERS_RESTORE,
            BANNERS_NOTE,
            SNACKBARS_LABEL,
            SNACK_SAVE_TRIGGER,
            SNACK_COPY_TRIGGER,
            SNACK_SUBMIT_TRIGGER,
            SNACK_ARCHIVE_TRIGGER,
            SNACK_SYNC_TRIGGER,
            SNACK_SAVED,
            SNACK_LINK_COPIED,
            SNACK_SUBMITTED,
            SNACK_VIEW_ACTION,
            SNACK_ARCHIVED_TITLE,
            SNACK_ARCHIVED_LINE,
            SNACK_UNDO_ACTION,
            SNACK_SYNC_FAILED_TITLE,
            SNACK_SYNC_FAILED_LINE,
            SNACK_RETRY_ACTION,
            SNACK_RESULT_RESTORED,
            SNACK_RESULT_RUNNING,
            SNACK_RESULT_OPENING,
            SNACKBARS_NOTE,
        ];
        texts.extend(TABS);
        texts.extend(STATE_CAPTIONS);
        for (_, sample) in SPECIMEN {
            texts.push(sample);
        }
        texts.extend(CHIP_FILTERS);
        for (label, _) in BADGES {
            texts.push(label);
        }
        texts.extend(REGIONS);
        texts.extend(OFFICES);
        texts.extend(TEAM_SIZES);
        texts.extend(CURRENCIES);
        texts.extend(PRACTICE_AREAS);
        texts.extend(START_OPTIONS);
        texts
    }

    /// Every gallery copy constant passes the voice lint: no em
    /// dashes, no stray ellipses, no exclamation marks, no emoji or
    /// out-of-range glyphs, none of the forbidden vocabulary, no
    /// fully title-cased headings past three words.
    #[test]
    fn gallery_copy_passes_the_voice_lint() {
        for text in all_copy() {
            let failures = voice_failures(text, false);
            assert!(failures.is_empty(), "{text:?}: {failures:?}");
        }
    }

    /// The component copy tables — the cover, the themes showcase and
    /// the brand showcase — pass the same lint, the Don't strings
    /// carrying their documented exemption from the vocabulary and
    /// sentence-case checks alone.
    #[test]
    fn component_copy_passes_the_voice_lint() {
        use crate::components::brand_showcase::{
            HEADLINES_HEADING, HEADLINE_MODES, PHOTO_HEADING, PHOTO_INTRO, PHOTO_NOTE, PHOTO_TILES,
            PRINCIPLE_BODY, PRINCIPLE_EYEBROW, PRINCIPLE_HEADLINE, PRINCIPLE_NUMERAL,
            SECTION_EYEBROW as BRAND_EYEBROW, VOICE_HEADING, VOICE_INTRO, VOICE_POSITIONS,
        };
        use crate::components::cover::{CAPTION, EYEBROW, HEADLINE, STANDFIRST};
        use crate::components::themes_showcase::{
            CARDS, CHIP_LABEL, DARK_ROW, INTRO, LIGHT_ROW, NOTE, PRIMARY_ACTION, SECONDARY_ACTION,
            SECTION_EYEBROW as THEMES_EYEBROW, SPECIMEN_LINE,
        };

        let mut texts = vec![
            BRAND_EYEBROW,
            PRINCIPLE_EYEBROW,
            PRINCIPLE_NUMERAL,
            PRINCIPLE_HEADLINE,
            PRINCIPLE_BODY,
            VOICE_HEADING,
            VOICE_INTRO,
            HEADLINES_HEADING,
            PHOTO_HEADING,
            PHOTO_INTRO,
            PHOTO_NOTE,
            EYEBROW,
            HEADLINE,
            STANDFIRST,
            CAPTION,
            THEMES_EYEBROW,
            INTRO,
            NOTE,
            LIGHT_ROW,
            DARK_ROW,
            SPECIMEN_LINE,
            PRIMARY_ACTION,
            SECONDARY_ACTION,
            CHIP_LABEL,
        ];
        for (_, register, example) in VOICE_POSITIONS {
            texts.push(register);
            texts.push(example);
        }
        for (label, example) in HEADLINE_MODES {
            texts.push(label);
            // The Don't strings are quoted anti-patterns: not the
            // kit's prose, exempt from the vocabulary and
            // sentence-case checks, held to the punctuation checks.
            let failures = voice_failures(example, label.starts_with("Don't"));
            assert!(failures.is_empty(), "{example:?}: {failures:?}");
        }
        for (_, caption) in PHOTO_TILES {
            texts.push(caption);
        }
        for card in CARDS {
            texts.push(card.character);
            texts.push(card.faces);
            // The legends are paint names and shares, not prose
            // (documented at the CARDS table).
            for entry in card.legend {
                let failures = voice_failures(entry, true);
                assert!(failures.is_empty(), "{entry:?}: {failures:?}");
            }
        }
        for text in texts {
            let failures = voice_failures(text, false);
            assert!(failures.is_empty(), "{text:?}: {failures:?}");
        }
    }

    /// The lint catches what it claims: each violation type, alone,
    /// produces its failure. The canary that proved the lint red
    /// before the copy was fixed, kept so the guarantees stay tested.
    #[test]
    fn the_lint_catches_what_it_claims() {
        assert!(!voice_failures("Sign up today!", false).is_empty());
        assert!(!voice_failures("One thing — then another", false).is_empty());
        assert!(!voice_failures("Loading…", false).is_empty());
        assert!(voice_failures(BUTTON_BUSY, false).is_empty());
        assert!(!voice_failures("Launch \u{1F680} now", false).is_empty());
        assert!(!voice_failures("Unlock your potential", false).is_empty());
        assert!(!voice_failures("World-Class Results Every Time", false).is_empty());
        // The exemption is narrow: it lifts the vocabulary and
        // sentence-case checks alone, never the punctuation checks.
        assert!(voice_failures("Strategic Outcomes. Engineered Trust.", true).is_empty());
        assert!(!voice_failures("Innovation unleashed!", true).is_empty());
        // The separator, the accents and the quotes the copy uses.
        assert!(voice_failures("Field notes · Gothenburg · cliché, Don't", false).is_empty());
    }
}
