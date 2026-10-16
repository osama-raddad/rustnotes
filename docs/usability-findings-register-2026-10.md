# Usability findings register, October 2026

Derived from the ledger `docs/usability-loop-2026-10.md`, with one row per distinct ID and the disposition checked against the current ledger. Line references are as of commit `b493b67`, and they go stale on the next ledger edit.
Earlier audit counts are superseded.

## Totals

- Distinct IDs: 148 (79 numbered, 20 D-items, 4 K-items, 45 descriptive). The 1-I2 row was added after the collapse.
- By severity: blocker 3; major 27; minor 44; nit 15; not stated 57.
- By final disposition: fixed 60; closed 15 (of which closed by design 7; K2 closed in iteration 10; two iOS right-margin rows closed in iteration 9); decision open 34 (6 of them with a default taken); open 39.

## Register

| ID | Severity | Surface | Summary | Final disposition | Where in the ledger |
|---|---|---|---|---|---|
| 1-A1 | major | gallery, Foundations states row | 'Start a project' clipped at both ends in all five tiles | fixed 67d5848 | Iter 1 L109-110; fix L171-174; device L198-201 |
| 1-A2 | major | gallery, Brand photo tiles | Captions overflow tiles; 'Gothenburg' clipped | fixed 339a1bf | Iter 1 L107-108; fix L168-170 |
| 1-A3 | major | gallery, page scroll | One page scroll offset shared by every panel | fixed 77e82bf | Iter 1 L102-104; fix L164-165 |
| 1-A4 | minor | gallery, Brand swatch band | Hairline seams cut every swatch; right edge fixed, seams kept as design (D2) | fixed e47784b | Iter 1 L126-127; Iter 2 L236-237; fix L358-360 |
| 1-A5 | minor | gallery, top bar | Wordmark sits above the bar's centre line | fixed 373819f | Iter 1 L128-129; Iter 2 L238-240; fix L361-363 |
| 1-A6 | minor | gallery, Brand single column | Tiles hug the left edge; right third empty. Column fill closed through 5-A3; left edge did not reproduce | fixed b1c5a6c | Iter 1 L141; Iter 2 L266-267; Iter 5 L713-717, L804-834 |
| 1-A7 | not stated | gallery, Brand captions | Dangling middle dot; orphan '1' fixed before iteration 2; dot residual fixed as 5-E1 | fixed 1581d7d | Iter 1 L130-131; Iter 2 residual L279-282; Iter 5 L835-855 |
| 1-A9 | not stated | gallery, Brand principle card | Principle card 01 about 443 pt tall (known follow-up from the Oct 14 pass) | open: no closure recorded | Iter 1 L142-143; schedule L57-60 |
| 1-A10 | not stated | gallery, Components | 'Book a call' shows a pressed state; other taps do nothing | decision open, default taken: specimens, which are inert (no D number; user to confirm) | Iter 1 L132-135 |
| 1-A11 | not stated | gallery, Foundations spacing and elevation | Staggered specimens; 'Radius Lg · 12' wraps; first tile border clipped | fixed 7e07d9b | Iter 1 L144-145; Iter 6 task 27 L904-910 |
| 1-A12 | not stated | ui-kit, colour-role swatches and eyebrow | Swatches invisible without an outline; eyebrow prints 1.4545455 | fixed db7c41b | Iter 1 L146-147; Iter 6 task 25 L892-896 |
| 1-A13 | not stated | gallery touch targets; Rust Notes cross and plus button | Touch targets under 44 pt. Pill still 25 pt (half fixed in fb99a5f as 2-A8); cross guarded in 7b07a78; plus button unguarded (known gap) | open: pill still 25 pt, under the 44-pt floor; plus button unguarded (known gap) | Iter 1 L136-140; Iter 6 review L874-876; Iter 7 L1026-1031; Status L1177 |
| 1-I1 | major | gallery, Theme popover toggle | OFF knob hangs outside the track; ON knob stops mid-track | fixed 4ca8a35 | Iter 1 L105-106; fix L166-167 |
| 1-I2 | minor | gallery, Brand hero band | Last swatch ends 23 px short of the margin; seams kept as a design element (D2) | fixed e47784b | Iter 1 L126-127; Iter 2 L236-237 |
| 1-I3 | not stated | gallery, top bar | Trailing gap wider than the leading gap | decision open: D3 | Iter 1 L128-129; Iter 2 L238-240; D3 L381-387 |
| 1-I4 | not stated | gallery, Theme popover rows | Rows 32 pt apart; toggles 7 pt off the radio column; labels 28 pt off | fixed ca1b391 | Iter 1 L148-149; moved to Iter 3 L405; fix L492-494 |
| 1-I5 | not stated | gallery on iOS, chrome | 28% of the iOS screen is chrome before any content; Android check first | open: iOS pass (iteration 9) partial; the iOS gap measures 56.3 pt, the same as Android's 1-I5 gap | Iter 1 L150-151; Iter 9 iOS results |
| 1-I7 | nit | gallery, Foundations | Specimen clipped at the fold | open: no closure recorded | Iter 1 L152-153 |
| 1-I8 | nit | gallery, Theme popover placement | Popover placement | fixed caa29e4 | Iter 1 L152-153; moved to Iter 3 L405; fix L502-506 |
| 1-I9 | nit | Rust Notes, date alignment | Date right-aligned and floating; guard covers the list at rest only | closed: guard test in 7b07a78 | Iter 1 L154; Iter 7 L1033-1037; Status L1176-1178 |
| 1-R1 | blocker | host, relaunch after Back | Relaunch in the same process freezes on the splash; same finding as K3 and D1 | decision open: D1 | Iter 1 L111-119; K3 L189-194; D1 L371-375 |
| 1-R2 | major | gallery, search popover and Back | Back with the search popover open exits the app; seen once, not reproduced | open: no closure recorded | Iter 1 L120-123; Iter 3 closure L483-573 does not mention Back |
| 1-R3 | not stated | gallery, search popover | Magnifier overlaps the placeholder; popover 6 px from the edge. Magnifier fixed in 43bc8c8 (2-A2) | fixed caa29e4 | Iter 1 L155-156; moved to Iter 3 L405; fix L451-452, L502-506 |
| 2-A1 | major | gallery, Components metric cards | Card 2 breaks mid-word; card 3 one letter per line; uneven heights accepted (L301-304) | fixed 688e280 | Iter 2 L229-231; task 6 L299-304; closed L349-352 |
| 2-A2 | major | gallery, text inputs | No left padding; magnifier overlaps the placeholder | fixed 43bc8c8 | Iter 2 L232-233; task 7 L305-310; closed L353-354 |
| 2-A3 | major | gallery, select popover | Popover stays open after an outside tap (mouse click too) | fixed 0ac89f8 | Iter 2 L234-235; task 8 L311-317; closed L355-357 |
| 2-A4 | minor | gallery, keyboard dismiss | Blank strip after the keyboard closes | closed: confirmed working in iteration 5 (no blank band after an option is chosen, two runs) | Iter 2 L244-245; Iter 5 L698-700 |
| 2-A5 | minor | gallery, top bar wordmark | Wordmark about 10 pt above the bar's centre line | fixed 373819f | Iter 2 L238-240; closed L361-363 |
| 2-A6 | minor | gallery, control hit areas | Chips 31 pt, radios 32, checkboxes 35, select options 33, Small button 36: all under 44 pt | decision open: D11 | Iter 2 L246-249; D11 L723-729 |
| 2-A7 | minor | gallery, Brand tab focus ring and tab order | Ring clipped at the left edge; tab order Theme before Search (tab order fixed in 12c256e) | fixed 206daff | Iter 2 L250-253; Iter 3 L441-443, L495-501 |
| 2-A8 | minor | gallery, Tab-to-me pill | About 17 pt tall with label touching the outline; now 25 pt, still under the 44-pt floor (D11, 1-A13) | fixed fb99a5f | Iter 2 L254-255; Iter 6 task 28 L911-913; review L874-876 |
| 2-A9 | nit | gallery, Disabled and Sending labels | 'Disabled' 1.1:1 is closed by design (38% documented opacity); 'Sending' label fixed here | fixed cbd4086 | Iter 2 L256-260; Iter 6 task 24 L886-891 |
| 2-A10 | minor | gallery, Components cards | Filled card has no visible surface; card widths vary | decision open: D12 | Iter 2 L261-262; D12 L730-733 |
| 2-A11 | minor | gallery, right gutter | Gutter 6-7 pt against 16 pt on the left | fixed b47473f | Iter 2 L263-265; moved to Iter 5 L406-407; task 20 L749-767 |
| 2-A12 | minor | gallery, Brand single column | Narrow columns; right third empty (same item as 1-A6) | fixed b1c5a6c | Iter 2 L266-267; Iter 5 L804-834 |
| 2-A13 | nit | gallery, glass specimens | Glass tile runs off its edge; shapes spill outside their tiles | open: no closure recorded | Iter 2 L268-269; Iter 5 L744-745; backlog Iter 10 L1121-1123 |
| 2-A15 | major | gallery, Region select | Opening the select scrolls the page about 96 pt | closed: not reproduced on the device in iteration 3 | Iter 2 L318-324; Iter 3 L430-432, L516-517 |
| 2-I1 | major | gallery, Feedback snackbar | 'Saved.' stays past its 6 s timeout until the next touch | fixed d64bb06 | Iter 2 L225-228; task 5 L290-298; closed L346-348 |
| 2-I2 | minor | gallery, Brand swatch hairlines | Three surface-coloured hairlines cut every swatch | closed by design: hairlines kept as the documented 'ruled stock' design element (D2 default kept) | Iter 2 L236-237; closed L358-360; D2 L377-380 |
| 2-I3 | minor | gallery, top bar | Trailing gap wider than the leading gap | decision open: D3 | Iter 2 L238-240; D3 L381-387 |
| 2-I4 | minor | ui-kit, wordmark rendering | Wordmark edges hard, with no anti-aliasing | fixed d660dbb | Iter 2 L241-243; Iter 6 task 26 L897-903 |
| 2-I5 | nit | gallery, busy label | 'Sending' active-state label reads about 1.2:1 | fixed cbd4086 | Iter 2 L256-260; Iter 6 task 24 L886-891 |
| 2-I8 | minor | gallery on iOS, bottom clip | Content hard-clipped near 816 pt; snackbar corners look notched | decision open: D20 (the scroll's bottom edge; 816 pt on iOS, 949 pt on Android); snackbar corners not checked | Iter 2 L270-272; Iter 2 closed L413-414; Iter 9 iOS results |
| 2-I9 | minor | gallery on iOS, bar edge and gap | Bar's bottom edge fuses with the divider; about 55 pt empty above the tab labels | open: the fused divider does not reproduce on iOS; the gap waits on 1-I5 | Iter 2 L273-275; Iter 2 closed L413-414; Iter 9 iOS results |
| ios-caret-under-keyboard | not stated | Rust Notes editor on iOS, long note | After a touch at the end, the caret stays under the keyboard until the first keystroke; the editor does not scroll to the caret when the body shrinks | open: minor, core scroll-to-caret; fix not started | Iter 9 iOS results |
| 3-A1 | major | gallery, Theme popover rows | Radio rows 32 pt apart, under the 44-pt floor; toggle labels 28 pt right of radio labels | fixed ca1b391 | Iter 3 L436-438; fix L492-494 |
| 3-A2 | major | gallery, top bar tab order | Tab goes Theme then Search; Search sits left of Theme on screen | fixed 12c256e | Iter 3 L439-440; fix L495-498 |
| 3-A3 | major | gallery, Brand tab focus ring | Ring clipped on its left edge at the content margin | fixed 206daff | Iter 3 L441-443; fix L499-501 |
| 3-A4 | minor | gallery, popover anchoring | Popover tops inside the bar border; Theme popover left edge aligned to nothing | fixed caa29e4 | Iter 3 L447-450; fix L502-506 |
| 3-A5 | minor | gallery, search popover padding | About 5 pt inside its edge, against 16 pt in the Theme popover | fixed caa29e4 | Iter 3 L451-452; fix L502-506 |
| 3-A6 | minor | gallery, search popover | Popover covers the tab strip on a phone | decision open: D6 | Iter 3 L462-464; D6 L533-535 |
| 3-A7 | minor | gallery, top bar bottom edge | Full-width hairline under the card border reads as a double rule | fixed ab9dfdd | Iter 3 L456-458; fix L512-515 |
| 3-A8 | minor | gallery, form fields and right gutter | Form fields overhang the right gutter (same item as 2-A11) | fixed b47473f | Iter 3 L474-475; task 20 L749-767 |
| 3-A9 | minor | gallery, page end after banner dismiss | Stale over-scroll leaves blank space until the next input | fixed 7bd2365 | Iter 3 L453-455; fix L507-511 |
| 3-A10 | nit | gallery, snackbar and paragraph spacing | Snackbar and paragraph spacing | open: no closure recorded | Iter 3 L473 |
| 3-A11 | minor | gallery, top bar bottom edge | Grouped with 3-A7 in the ledger | fixed ab9dfdd | Iter 3 L456-458; fix L512-515 |
| 3-A12 | nit | gallery, Theme chevron | Grouped with 3-I4 in the ledger | decision open: D7 | Iter 3 L465-467; D7 L536-537 |
| 3-I1 | major | gallery, Theme popover toggles | Grouped with 3-A1 in the ledger | fixed ca1b391 | Iter 3 L436-438; fix L492-494 |
| 3-I2 | minor | gallery, popover anchoring | Grouped with 3-A4 in the ledger | fixed caa29e4 | Iter 3 L447-450; fix L502-506 |
| 3-I3 | minor | gallery, Theme popover edge | Right edge 8 pt inside the bar edge; grouped with 3-A4 | fixed caa29e4 | Iter 3 L447-450; fix L502-506 |
| 3-I4 | nit | gallery, Theme chevron | Chevron does not flip when the menu opens | decision open: D7 | Iter 3 L465-467; D7 L536-537 |
| 3-I6 | minor | gallery, search popover | Grouped with 3-A6 in the ledger | decision open: D6 | Iter 3 L462-464; D6 L533-535 |
| 3-I7 | minor | gallery, search popover padding | Grouped with 3-A5 in the ledger | fixed caa29e4 | Iter 3 L451-452; fix L502-506 |
| 3-I8 | nit | gallery, snackbar | Snackbar covers its trigger row | closed by design: a snackbar floats over content by design | Iter 3 L468-469 |
| 3-I9 | nit | gallery, Restore the banners link | No affordance on the link | open: no closure recorded | Iter 3 L470-472; backlog Iter 10 L1121-1123 |
| 3-I10 | nit | gallery, banner dismiss glyph | Bare 12-pt glyph (hit area about 36 pt) | open: no closure recorded | Iter 3 L470-472; backlog Iter 10 L1121-1123 |
| 3-I11 | not stated | gallery, top bar bottom edge | Grouped with 3-A7 and 3-A11 in the ledger fix entry | fixed ab9dfdd | Iter 3 L512-515 |
| 4-A1 | major | gallery, Theme popover in landscape | Overflows the bottom with no scroll area; last row unreachable | fixed 72ab126 | Iter 4 L590-592; fix L619-624 |
| 4-A2 | major | gallery, page area in landscape | Scroll area about 109 pt of a 448 pt screen | fixed 66cda43 | Iter 4 L593-595; fix L625-628 |
| 4-A3 | minor | gallery, one-line banner | Keeps a two-line height; dismiss X sits low | open: no closure recorded | Iter 4 L603-604; listed for Iter 5 L667-668, not recorded there |
| 4-A4 | minor | Rust Notes, editor with keyboard in landscape | Editor shows about 1.5 lines with the keyboard up | closed by design: keyboard's own height; caret stays visible; no layout defect | Iter 4 L604-606; Iter 7 L981-983, L1039-1041 |
| 4-A5 | major | Rust Notes, landscape inset | Content ran 208 px from the left and 48 px from the right | fixed 0de8ad4 | Iter 4 L596-599; fix L629-632 |
| 5-A1 | major | gallery, Components right gutter | Text fields and horizontal scrollers run past the bar's right edge | fixed b47473f | Iter 5 L705-709; task 20 L749-767 |
| 5-A2 | major | gallery, Feedback tab at font scale 1.3 | Selected tab cut at the strip edge and not scrolled into view | fixed 7f8ed7c | Iter 5 L710-712; task 21 L778-802 |
| 5-A3 | major | gallery, Brand column | Theme, voice, photo and Do/Don't cards fill only part of the column | fixed b1c5a6c | Iter 5 L713-717; task 22 L804-834 |
| 5-C1 | not stated | gallery, Foundations elevation tiles | Tiles staggered (tops 54 to 101 pt) | fixed 7e07d9b | Iter 5 L739-741; Iter 6 task 27 L904-910 |
| 5-E1 | minor | gallery, photo caption | Middle dot ends a caption row | fixed 1581d7d | Iter 5 L718-719; task 23 L835-855 |
| 7-A1 | major | Rust Notes, swipe-to-delete | Swipe deleted on release with no confirmation or undo | fixed beeec62 | Iter 7 L957-962; Status L1174-1175 |
| 7-A2 | major | Rust Notes, landscape editor | Editor text started about 140 pt in; cap scope recorded under D16 (458f343) | fixed 32b3f86 | Iter 7 L969-975; Status L1175-1176 |
| 7-A3 | major | Rust Notes, backgrounding | Edits under 0.5 s before Home lost after a process kill | fixed 9188861 | Iter 7 L985-1011; Status L1176-1177 |
| D1 | blocker | host, relaunch after Back | Relaunch freeze after Back at the gallery root; testers use am force-stop | decision open: D1 | Iter 1 L111-119; Iter 2 L371-375; Status L1211-1212 |
| D2 | not stated | gallery, Brand swatch band | Three surface-coloured hairlines across the band | decision open, default taken: kept as a documented design element | Iter 2 L377-380; Status L1213 |
| D3 | not stated | gallery, top bar | Trailing gap on the Theme trigger | decision open: D3 | Iter 2 L381-387; Status L1214 |
| D4 | not stated | gallery, wordmark | Wordmark reads 'Vnatix' at a glance | decision open, default taken: intentional monogram | Iter 2 L388-391; Status L1215 |
| D5 | not stated | personal iPhone | Vynatix Gallery installed on the connected iPhone 17 Pro | decision open: D5 | Iter 2 L392-396; Status L1216-1217 |
| D6 | not stated | gallery, search popover | Covers the tab strip on a phone | decision open: D6 | Iter 3 L533-535; Status L1218 |
| D7 | not stated | gallery, Theme chevron | Does not flip when the menu opens | decision open: D7 | Iter 3 L536-537; Status L1218 |
| D8 | not stated | gallery, Theme toggles | Toggle glyphs about 24 pt left of the radio circles | decision open: D8 | Iter 3 L521-523, L538-539; Status L1218 |
| D9 | not stated | gallery, search popover | Held at the bar's left edge on a phone instead of hung from its trigger | decision open: D9 | Iter 3 L525-527, L540-541; Status L1218 |
| D10 | not stated | gallery, landscape side margins | Margins are canvas, not bar colour; full-bleed bars would be a different change | decision open: D10 | Iter 4 L634-639; Status L1218-1219 |
| D11 | not stated | gallery, kit controls | 44-pt touch floor, kit-wide; blocks the touch-target sweep | decision open: D11 | Iter 5 L723-729; Status L1220 |
| D12 | not stated | gallery, Components cards | Specimens use about 73% of the column; filled card has no surface | decision open: D12 | Iter 5 L730-733; Status L1221 |
| D13 | not stated | gallery, phone photo tiles | One per row at full width, or two-up | decision open, default taken: one per row (two-up is the alternative) | Iter 5 L820-824; Status L1222 |
| D14 | not stated | gallery, photo captions | A caption's middle dot can begin a line after the no-break fix | decision open: D14 | Iter 5 L845-850; Status L1223 |
| D15 | not stated | ui-kit, WidgetState | Cannot tell busy from disabled | decision open: D15 | Iter 6 L889-891; Status L1224 |
| D16 | major | Rust Notes, editor width cap | Keep the reading width on Medium portrait, or go full width; cap fixed in 458f343 | decision open, default taken: keep the reading width on Medium portrait (current) | Iter 7 L976-980; Iter 10 L1153-1154; Status L1225-1236 |
| D17 | minor | Rust Notes, swipe hint | Wording does not name the swipe; tapping it opens the note | decision open: D17 | Iter 10 L1155-1156; Status L1237-1239 |
| D18 | major | gallery, icons row | Top or bottom alignment for the icons row | decision open: D18 | Iter 10 L1146-1147; Status L1240-1242 |
| D19 | not stated | gallery, OS night flag | Gallery starts on Forest Light whatever the OS night flag says | decision open: D19 | Iter 6 L928-931; Status L1243-1244 |
| D20 | not stated | gallery, scroll bottom edge | Scroll viewport ends 58 pt above the screen bottom (34 pt safe area + 24 pt panel margin); the Pixel's ends at 949 pt by the same structure | decision open: D20 (default taken: keep) | Iter 9 iOS results; Status Open decisions |
| K1 | not stated | ui-kit and gallery, touch input | Rightward pointer jump in the first touch frame moves a clamped specimen row | open: known gap; no fix or ignored regression test recorded | Iter 1 L180-185; Known gaps L1249 |
| K2 | not stated | gallery, axis-lock coverage | Mutation check denied by the classifier; coverage unverified | closed in iteration 10: mutation caught at gallery.rs:405, input_tests.rs:509 fired, clean revert | Iter 1 L186-189; Iter 10 K2 entry; Known gaps |
| K3 | blocker | host, relaunch after Back | Same issue as 1-R1 and D1 | decision open: D1 | Iter 1 L189-194; D1 L371-375 |
| K4 | not stated | tools/emu-touch.sh, unswipe check | Failed in 3 of 12 runs over four batches; restored row settles short; cause not traced | open: informational; no more runs until the short state is diagnosed | Iter 10 K4 entry; Known gaps |
| 7-hold-to-select | not stated | Rust Notes, editor | Hold then drag selects text; no word-select or selection handles | open: no closure recorded (observation, not a fix) | Iter 7 L1012-1016 |
| branch-core-editor-cap | major | Rust Notes, editor width cap | Cap removed for every single-pane window, contrary to metrics.rs:24; Resume step 1 still lists finishing the scoped fix | fixed 458f343 | Iter 10 L1153-1154, L1197-1199; D16 L1225-1226; Resume L1258 |
| branch-core-swipe-hint-tap | minor | Rust Notes, swipe hint | Hint can be tapped; it opens the note and drops the arm | decision open: D17 | Iter 10 L1155-1156, L1205-1206; D17 L1237-1239 |
| branch-core-two-pane-stale-frame | minor | Rust Notes, two-pane detail | Same as core-two-pane-stale-frame | open: no closure recorded (backlog) | Iter 10 L1155-1157 |
| branch-gallery-harness-855pt | minor | gallery harness | Same as gallery-harness-855pt | open: no closure recorded (backlog, no fix date) | Iter 10 L1148-1149, L1201-1202 |
| branch-gallery-icons-alignment | major | gallery, icons row | Top-aligned by the 7e07d9b wrapping_row change; no test covers it | decision open: D18 | Iter 10 L1146-1147, L1194-1196; D18 L1240-1242 |
| branch-gallery-theme-menu-width | minor | gallery, Theme popover width | Same as gallery-popover-width-ceiling | open: no closure recorded (backlog, not run) | Iter 10 L1149-1151 |
| branch-ui-tab-strip-motion | minor | ui-kit, tab strip scroll | Same as ui-kit-tab-strip-reduced-motion | open: no closure recorded (backlog, no fix date) | Iter 10 L1142-1143, L1200 |
| components-glass-spill | nit | gallery, Components | Glass specimen spills on the Components page (same as 2-A13) | open: no closure recorded | Iter 10 L1121-1123; Iter 2 L268-269 |
| components-restore-banners-link | nit | gallery, Restore the banners link | No affordance (same as 3-I9 and 3-I10) | open: no closure recorded | Iter 10 L1121-1123 |
| core-detail-edge-left-inset | minor | Rust Notes, detail right edge | May not match the mirrored bar edge in a left-only inset; low reachability | open: no closure recorded (backlog, low reachability) | Iter 10 L1157-1158 |
| core-two-pane-stale-frame | minor | Rust Notes, two-pane detail | Stale frame after a cross delete; predates the branch; not run on a device | open: no closure recorded (backlog) | Iter 10 L1155-1157, L1206-1207 |
| focus-ring-theme-trigger | not stated | gallery, Theme trigger focus ring | Ring clipped by the overhang (regression from the iteration 2 work) | fixed 747dcfd | Iter 2 L364-367 |
| foundations-elevation-spacing | not stated | gallery, Foundations elevation specimens | Specimens staggered with gaps of about 100 pt | fixed 7e07d9b | Iter 5 L739-741; Iter 6 task 27 L904-910 |
| gallery-harness-855pt | minor | gallery harness | Paints only the first 855 pt, so gutter tests cover the first screen only | open: no closure recorded (backlog, no fix date) | Iter 10 L1148-1149, L1201-1202 |
| gallery-popover-width-ceiling | minor | gallery, Theme popover width | THEME_MENU_WIDTH (248 pt) can pass the trigger's right edge at 265 to 295 pt windows; not run | open: no closure recorded (backlog, not run) | Iter 10 L1149-1151, L1202-1204 |
| gallery-tab-strip-test-doc | minor | gallery, tab-strip test | Test doc says it follows a tap; it sets the host's selection | open: no closure recorded (backlog, no fix date) | Iter 10 L1151-1152 |
| gesture-bar-blank-band | not stated | gallery, Components end, Android and iOS | Blank band above the gesture bar; hard clip at the bottom inset (iOS 816 pt) | open: iOS clip half not measured (iteration 9 partial); Android band is backlog, not a defect | Iter 5 L736-739; Iter 9 L1111-1117 |
| iter3-banners-label-at-page-end | not stated | gallery, Banners label | Label sits half under the tab strip at the scroll limit | open: no closure recorded (open question for the user; no D number) | Iter 3 L542-545 |
| iter3-editorial-strip-swipe | not stated | gallery, editorial strip | Strip scrolled instead of the page, once; may be the PR #23 horizontal-drag issue | closed: not reproduced in iteration 3 | Iter 3 L476-478, L566-568 |
| iter4-duplicated-tabs-landscape | not stated | gallery, top bar and tab strip | Same four tabs repeated in landscape; portrait only checked | open: no closure recorded | Iter 4 L652-653; Iter 5 L700-701 |
| iter4-not-checked | not stated | gallery Foundations in landscape; Rust Notes Settings; other devices | Checks not made on the single-cutout emulator | open: no closure recorded | Iter 4 L607-609 |
| iter4-two-pane-side-cutout | not stated | Rust Notes, two-pane with side cutout | Surface may show in the detail pane's right margin; headless check only, device needs a 480 pt cutout phone | fixed 99a3ad1 | Iter 4 L646-648; Iter 8 L1062-1068; Status L1180-1182 |
| iter6-bdd9607-commit-hygiene | not stated | branch history | Commit bdd9607 (titled for task 24) also carries a task-25 hunk; history not rewritten | open: split, or disclosure in the PR, not recorded | Iter 6 L879-884; Resume L1259-1260 |
| iter6-review-nits-27-28 | nit | gallery, stagger and pill tests | Review nits for tasks 27 and 28 not yet made | open: no closure recorded | Iter 6 L873-876 |
| iter6-task29-ratio-bar | not stated | gallery, Teal Light card ratio bar | Bar border still outline_variant (1.31:1 on Teal Light); unclear whether the finding meant the bar | open: unclear whether the finding meant the bar; bar unchanged | Iter 6 L919-921, L933-934; Status L1245 |
| iter6-task29-teal-swatch-borders | not stated | gallery, Teal Light card swatches | Swatch borders now outline (Teal Light 4.70:1) | fixed b8d4cb9 | Iter 6 L914-918; device L932-934 |
| iter8-portrait-editor-title-uncapped | not stated | Rust Notes, portrait editor | Title bar uncapped while the body is capped to a centred 640 column | closed by design: scaffold caps the body only (scaffold.rs:174); bar stays full width | Iter 8 L1058-1060; D16 disposition L1229-1231 |
| iter8-two-pane-surface-inset | not stated | Rust Notes, two-pane detail surface | Surface ran past the scaffold's mirrored edge; headless check only | fixed 99a3ad1 | Iter 8 L1062-1068; Status L1180-1182 |
| iter9-blank-band-above-gesture-bar | not stated | gallery Components end, Android | Blank band of 68.7 pt above the gesture bar | closed: not a defect; recorded as backlog, the space keeps the last card clear of the bar | Iter 9 L1111-1117 |
| iter9-components-right-margin-ios | not stated | gallery on iOS, Components right margin | Right margin unmeasured; screenshot check inconclusive | closed: symmetric margin (divider 16.0 to 386.0 pt on iOS); icon and chips rows end at their natural widths; gutter tests green at 400 and 416 pt | Iter 9 iOS results |
| iter9-ios-bottom-clip | not stated | gallery on iOS, bottoms of Components and Feedback | Hard clip near 816 pt reported by an earlier tester; not measured | open: iOS pass (iteration 9) is partial; page never scrolled to its end | Iter 9 L1101-1104; Status L1183-1186 |
| iter9-keyboard-inset | not stated | Rust Notes editor on iOS, keyboard | Whether the caret stays above the keyboard | open: iOS pass (iteration 9) is partial; not checked | Iter 9 L1108-1110; Status L1184-1186 |
| iter9-keyboard-inset-ios | not stated | Rust Notes editor on iOS, keyboard | Same item as iter9-keyboard-inset | open: iOS pass (iteration 9) is partial; not checked | Iter 9 L1108-1110 |
| iter9-status-bar-vynatix-gallery-link | not stated | iOS status bar | 'Vynatix Gallery' back-to-app link shown during the iOS run | closed by design: iOS's own back-to-app link, not a Rust Notes defect | Iter 9 L1098-1100 |
| snackbar-over-trigger-row | not stated | gallery, snackbar | Floats over its own trigger row (cosmetic) | closed by design: a snackbar floats over content by design | Iter 2 L403-404; Iter 3 L468-469 |
| start-a-project-tap | not stated | gallery, Components | 'Start a project' gives no visible response to a tap | closed by design: demo button with no action; its pressed look is the feedback | Iter 2 L276-278 |
| tab-to-me-pill-height | not stated | gallery, Tab-to-me pill | 18 pt tall with about 3 pt of padding; cramped. Now 25 pt, still under the 44-pt floor (D11) | fixed fb99a5f | Iter 5 L742-743; Iter 6 task 28 L911-913 |
| task-20-ios-caveat | not stated | gallery on iOS, Components right margin | Right margin left as 'iOS is not claimed' by task 20 | closed: symmetric margin on iOS; icon and chips rows end at their natural widths | Iter 5 L765-767; Iter 9 iOS results |
| teal-light-ratio-bar-border | not stated | gallery, Teal Light card ratio bar | Same ratio bar border, 1.31:1 on Teal Light | open: unclear whether the finding meant the bar; bar unchanged | Iter 6 L919-921; Status L1245 |
| top-bar-bottom-hairline | minor | gallery, top bar | Bottom hairline runs past the rounded corners | fixed ab9dfdd | Iter 2 L402-404; Iter 3 L456-458, L512-515 |
| two-pane-surface-inset | not stated | Rust Notes, two-pane detail surface | Same finding as iter8-two-pane-surface-inset, under a second ID | fixed 99a3ad1 | Iter 8 L1062-1068 |
| ui-kit-outline-logo-comments | not stated | ui-kit, outline.rs:26 and logo.rs:260 comments | Two code comments are wrong, not the code | open: no closure recorded (backlog, no fix date) | Iter 10 L1143-1145 |
| ui-kit-tab-strip-reduced-motion | minor | ui-kit, tab strip scroll | Scroll ignores reduced motion; a glide still runs | open: no closure recorded (backlog, no fix date) | Iter 10 L1142-1143, L1200 |

## Remaining work

Open and decision-open rows, grouped by the iteration where each was raised.

### Iteration 1
- 1-R1 (blocker): decision open: D1, the relaunch freeze after Back. Ledger L111-119, D1 L371-375.
- 1-R2 (major): open: no closure recorded, Back with the search popover open exits the app. Ledger L120-123.
- 1-I3: decision open: D3, the trailing gap. Ledger L128-129, D3 L381-387.
- 1-A9: open: no closure recorded, principle card 01 about 443 pt tall. Ledger L142-143.
- 1-A10: decision open, default taken: specimens, which are inert. Ledger L132-135.
- 1-A13: open: pill still 25 pt, under the 44-pt floor; plus button unguarded (known gap). Ledger L136-140, L874-876, L1029-1031.
- 1-I5 (iOS): open: iOS gap measured at 56.3 pt; closure is a decision on the chrome. Ledger L150-151.
- 1-I7 (nit): open: no closure recorded, specimen clipped at the fold. Ledger L152-153.
- K1: open: known gap, rightward pointer jump; no fix or ignored regression test. Ledger L180-185.
- K2: closed in iteration 10: mutation caught. Ledger iteration 10 K2 entry.
- K3 (blocker): decision open: D1. Ledger L189-194.
- D1 (blocker): decision open: D1, relaunch freeze after Back. Ledger L371-375.

### Iteration 2
- 2-A6 (minor): decision open: D11, control hit areas under 44 pt. Ledger L246-249.
- 2-A10 (minor): decision open: D12, card specimens. Ledger L261-262.
- 2-I3 (minor): decision open: D3, trailing gap. Ledger L238-240.
- 2-A13 (nit): open: no closure recorded, glass specimens; carried to the iteration 10 backlog. Ledger L268-269, L1121-1123.
- 2-I8 (minor, iOS): decision open: D20, the scroll's bottom edge (816 pt on iOS). Ledger L270-272, iteration 9 iOS results.
- 2-I9 (minor, iOS): open: fused divider does not reproduce on iOS; gap waits on 1-I5. Ledger L273-275.
- ios-caret-under-keyboard: open: minor, core scroll-to-caret on iOS. Iteration 9 iOS results.
- components-glass-spill (nit): open: no closure recorded, same as 2-A13. Ledger L1121-1123.
- D2: decision open, default taken: kept as a documented design element. Ledger L377-380.
- D3: decision open: D3. Ledger L381-387.
- D4: decision open, default taken: intentional monogram. Ledger L388-391.
- D5: decision open: D5, remove the gallery from the personal iPhone. Ledger L392-396.

### Iteration 3
- 3-A6 (minor): decision open: D6, search popover covers the tab strip. Ledger L462-464.
- 3-I6 (minor): decision open: D6, same as 3-A6. Ledger L462-464.
- 3-A12 (nit): decision open: D7, Theme chevron. Ledger L465-467.
- 3-I4 (nit): decision open: D7, same as 3-A12. Ledger L465-467.
- 3-I9 (nit): open: no closure recorded, Restore the banners affordance; iteration 10 backlog with no fix date. Ledger L470-472, L1121-1123.
- 3-I10 (nit): open: no closure recorded, banner dismiss glyph; iteration 10 backlog. Ledger L470-472, L1121-1123.
- 3-A10 (nit): open: no closure recorded, snackbar and paragraph spacing. Ledger L473.
- components-restore-banners-link (nit): open: no closure recorded, same as 3-I9 and 3-I10. Ledger L1121-1123.
- iter3-banners-label-at-page-end: open: no closure recorded, question for the user. Ledger L542-545.
- D6: decision open: D6. Ledger L533-535.
- D7: decision open: D7. Ledger L536-537.
- D8: decision open: D8, toggle glyphs versus labels. Ledger L521-523, L538-539.
- D9: decision open: D9, held search popover. Ledger L525-527, L540-541.

### Iteration 4
- 4-A3 (minor): open: no closure recorded, one-line banner keeps a two-line height. Ledger L603-604, L667-668.
- iter4-duplicated-tabs-landscape: open: no closure recorded, duplicated tabs in landscape. Ledger L652-653.
- iter4-not-checked: open: no closure recorded, Foundations in landscape and other devices. Ledger L607-609.
- D10: decision open: D10, full-bleed landscape bars. Ledger L634-639.

### Iteration 5
- task-20-ios-caveat: closed in iteration 9: symmetric margin on iOS. Ledger L765-767, iteration 9 iOS results.
- gesture-bar-blank-band: open: iOS clip half not measured; Android band is backlog, not a defect. Ledger L736-739, L1111-1117.
- D11: decision open: D11, 44-pt touch floor. Ledger L723-729.
- D12: decision open: D12, Components card widths. Ledger L730-733.
- D13: decision open, default taken: one per row (two-up is the alternative). Ledger L820-824.
- D14: decision open: D14, caption dot can start a line. Ledger L845-850.

### Iteration 6
- iter6-task29-ratio-bar: open: unclear whether the finding meant the bar; bar still uses outline_variant. Ledger L919-921, L933-934.
- teal-light-ratio-bar-border: open: same item as iter6-task29-ratio-bar. Ledger L919-921, L1245.
- iter6-bdd9607-commit-hygiene: open: split, or disclosure in the PR, not recorded. Ledger L879-884, L1259-1260.
- iter6-review-nits-27-28 (nit): open: no closure recorded, review nits not yet made. Ledger L873-876.
- D15: decision open: D15, WidgetState busy versus disabled. Ledger L889-891.
- D19: decision open: D19, OS night flag. Ledger L928-931.
- D20: decision open: the scroll's bottom edge (2-I8). Default taken: keep. Iteration 9 iOS results.

### Iteration 7
- 7-hold-to-select: open: no closure recorded, observation, not a fix. Ledger L1012-1016.
- D16 (major): decision open, default taken: keep the reading width on Medium portrait (current); cap fixed in 458f343. Ledger L976-980, L1225-1236.

### Iteration 9
- iter9-ios-bottom-clip: open: iOS pass (iteration 9) is partial; page never scrolled to its end. Ledger L1101-1104.
- iter9-components-right-margin-ios: open: iOS pass (iteration 9) is partial. Ledger L1105-1107.
- iter9-keyboard-inset: open: iOS pass (iteration 9) is partial; caret above keyboard not checked. Ledger L1108-1110.
- iter9-keyboard-inset-ios: open: same item as iter9-keyboard-inset. Ledger L1108-1110.

### Iteration 10
- K4: open: 3 of 12 runs fail unswipe; cause not traced; informational. Ledger iteration 10 K4 entry and known gaps.
- branch-gallery-icons-alignment (major): decision open: D18. Ledger L1146-1147, L1240-1242.
- D18 (major): decision open: D18, top or bottom alignment. Ledger L1240-1242.
- branch-core-swipe-hint-tap (minor): decision open: D17. Ledger L1155-1156, L1237-1239.
- D17 (minor): decision open: D17, swipe hint wording and tap. Ledger L1237-1239.
- ui-kit-tab-strip-reduced-motion and branch-ui-tab-strip-motion (minor): open: no closure recorded, backlog. Ledger L1142-1143, L1200.
- ui-kit-outline-logo-comments: open: no closure recorded, backlog. Ledger L1143-1145.
- gallery-harness-855pt and branch-gallery-harness-855pt (minor): open: no closure recorded, backlog. Ledger L1148-1149, L1201-1202.
- gallery-popover-width-ceiling and branch-gallery-theme-menu-width (minor): open: no closure recorded, backlog, not run. Ledger L1149-1151, L1202-1204.
- gallery-tab-strip-test-doc (minor): open: no closure recorded, backlog. Ledger L1151-1152.
- core-two-pane-stale-frame and branch-core-two-pane-stale-frame (minor): open: no closure recorded, backlog; not run on a device. Ledger L1155-1157, L1206-1207.
- core-detail-edge-left-inset (minor): open: no closure recorded, backlog, low reachability. Ledger L1157-1158.

### Resume here (handoff, ledger resume list)
- Decide D1 to D20. D16 has a default: keep the reading width on Medium portrait. D20 has a default: keep the bottom margin.
- Iteration 9 closed with one open minor (ios-caret-under-keyboard). The bottom clip is D20; the right margin is closed.
- Iteration 10's open items: K4 (diagnose the settled-short state), and the sequential backlog workflow. The HEAD sweep is done, with no regressions.
- Refresh PR #24's title and body (iterations 1 to 8 closed, 9 partial, 10 closed with open items, D1 to D19, K1 to K4, the register path).
- Re-run the trailer check over `162d7e7..HEAD`.

### Iteration 9 open items (ledger L1101-1110)
- iOS bottom clip at 816 pt (2-I8): cause established (874 − 34 − 24); decision D20.
- Right margin of the Components page on iOS (task-20-ios-caveat; iter9-components-right-margin-ios).
- Bar's edge and gap on iOS (2-I9).
- Keyboard inset: caret above the keyboard in the Rust Notes editor (iter9-keyboard-inset; iter9-keyboard-inset-ios).

### Iteration 10 open items (ledger L1119-1161, L1187-1190, L326-336)
- Backlog carried into iteration 10, with no fix date: Components glass specimen spill (2-A13; components-glass-spill) and the Restore the banners affordance (3-I9, 3-I10; components-restore-banners-link).
- K4: emulator touch script unswipe check; 3 of 12 runs fail; the restored row settles short; cause not traced.
- Branch review backlog: the minor and major items listed under the iteration 10 rows above, plus D17 and D18.
- D16: the scoped fix is made (`458f343`, reviewed PASS). The user's choice on Medium portrait remains: decision open, default taken (keep the reading width).
- Re-screenshot sweep at HEAD against the `iter-N-verify` baselines: not done.
- Test-quality backlog, not IDs (ledger L326-336, L414-415, L561-564, L771-776): metric-card padding round-trip and build helper; text-field wrap-width assertion and helper; painted_text and painted_strings shared walker; chips-row guard against CHIP_FILTERS drift; golden-table re-recording notes; shape walkers repeated across test files.

## Decisions for the user

- D1 (blocker): decision open. Relaunch freeze after Back at the gallery root; options are end the process at the root, or investigate re-entry. Ledger L371-375, L1211-1212.
- D2: decision open, default taken: kept as a documented design element. Ledger L377-380, L1213.
- D3: decision open. Trailing gap on the Theme trigger; no default stated. Ledger L381-387, L1214.
- D4: decision open, default taken: intentional monogram. Ledger L388-391, L1215.
- D5: decision open. Vynatix Gallery on the connected iPhone 17 Pro; no default stated. Ledger L392-396, L1216-1217.
- D6: decision open. Search popover covers the tab strip; no default stated. Ledger L533-535, L1218.
- D7: decision open. Theme chevron does not flip; no default stated. Ledger L536-537, L1218.
- D8: decision open. Toggle glyphs versus labels; no default stated. Ledger L521-523, L538-539, L1218.
- D9: decision open. Held search popover; no default stated. Ledger L525-527, L540-541, L1218.
- D10: decision open. Full-bleed landscape bars versus canvas margins; no default stated. Ledger L634-639, L1218-1219.
- D11: decision open. 44-pt touch floor, kit-wide; blocks the touch-target sweep. Ledger L723-729, L1220.
- D12: decision open. Components card widths; blocks the sweep. Ledger L730-733, L1221.
- D13: decision open, default taken: one per row (two-up is the alternative). Ledger L820-824, L1222.
- D14: decision open. A caption's dot can begin a line; no default stated. Ledger L845-850, L1223.
- D15: decision open. WidgetState cannot tell busy from disabled; no default stated. Ledger L889-891, L1224.
- D16: decision open, default taken: keep the reading width on Medium portrait (current); the cap itself was fixed in 458f343. Ledger L976-980, L1225-1236.
- D17: decision open. Swipe hint wording and tap behaviour; no default stated. Ledger L1155-1156, L1237-1239.
- D18: decision open. Icons row top or bottom alignment; no default stated. Ledger L1146-1147, L1240-1242.
- D19: decision open. Should the gallery follow the OS night flag at launch; no default stated. Ledger L928-931, L1243-1244.

## Known gaps

- K1: a rightward pointer jump in the first touch frame moves a clamped specimen row; no fix or ignored regression test recorded. Ledger L180-185, L1249.
- K2: closed in iteration 10. The axis-lock mutation is caught by the jump test (`gallery.rs:405`, `input_tests.rs:509`).
- K3: the relaunch freeze after Back (same as D1, and 1-R1 at blocker severity). Ledger L189-194.
- K4: the emulator touch script's unswipe check failed in 3 of 12 runs over four batches; cause not traced. Ledger iteration 10 K4 entry.

## Sources

- Ledger: `docs/usability-loop-2026-10.md`
- Assembled from an eight-section read of the ledger at `45be439`, then re-checked against the ledger at `b493b67`.
- Dispositions verified against the ledger at the head of the branch.
- Matching notes: compound audit entries such as "3-A1 / 3-I1" are split into their component IDs. IDs are kept as written; descriptive IDs use the slug without its parenthetical. Two IDs for one finding (iter8-two-pane-surface-inset and two-pane-surface-inset; iter9-keyboard-inset and iter9-keyboard-inset-ios) keep separate rows. Where audits reuse an ID for different text (1-A13, 2-A9, 2-I5, 1-I9), the ledger's own text decides.
- Where the ledger closes a finding under its named item rather than under the same ID, the closing record is cited (1-A6 through 5-A3; 1-A7 through 5-E1; 1-I4 and 1-I8 through iteration 3).
- Commit SHAs in the dispositions were checked with git cat-file against the worktree; all resolve.
- Ledger conflicts: D16 is "by design" with a question open in the Open decisions block, while Resume step 1 says the scoped fix is still to be finished; the register follows the Open decisions block. The ledger's top line says iteration 10 is closed with open items; its Where things stand says in progress. The register follows the iteration heading.
- 1-I2 was recorded in the ledger but in no audit; it now has its own row (grouped with 1-A4 in the ledger).
