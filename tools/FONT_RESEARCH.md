# Menu font investigation

`MenuGlyphs.swift` retains the original status/settings fallback. The app
also uses the Rust 8- and 12-pixel matchers described below; the font
coverage and layouts are not yet complete. The ROM-supplied 8x8 table at `0x1e5a84` is an
ASCII/CP437-style font: for example the digit zero starts at `0x1e5b04`
and uppercase L at `0x1e5be4`. Finding it does not establish that it is the
menu font.

`find_font_candidates.py` compares normalized menu samples to 8-byte 1-bpp
windows in a user-supplied ROM. For example:

```
python3 tools/find_font_candidates.py /path/to/firmware --start 0x1e5a84 --end 0x1e5e00 --stride 8
python3 -m unittest discover -s tools/tests
```

Scores are Hamming distances out of 144 normalized pixels. This is a search
heuristic, not a character mapping: unrelated ASCII characters can score
well. It assumes one byte per row, MSB first, and stretches bounding boxes;
other layouts, compression, antialiasing, and pre-rendered labels are not
covered. Confirm any proposed table against independently captured screens,
multiple characters and variants before using it at runtime. Do not commit
firmware or user save data.

Remaining work: identify and validate the actual font assets and mappings in
each supported ROM, implement segmentation and scale handling against real
screens, apply a verified local lexicon to ambiguous candidates, and test on
multiple screens and versions. No claim of complete recognition is justified
by the current samples.

## Emulator-derived samples

For the Japanese Sweets firmware, `じょうたい` occurs as big-endian game
character codes in flash. Editing that label in a *temporary snapshot* and
opening the menu makes the original renderer draw selected codes. With a
matching ROM, a room-screen snapshot, and a built `tg18cli`, reproduce the
samples with `capture_menu_font.py --rom ROM --snapshot SNAPSHOT --emulator
TG18CLI --output OUTPUT.json`. The script requires the label to be unique and
requires the expected 3x RGB PNG output; it never writes the input files.

`fixtures/sweets-menu-glyphs.json` contains 277 isolated, labelled glyphs and variants
captured this way, including the middle dot. The labels are from the existing
character-code table and should not be taken as independent confirmation of
every code. Ten independent menu screens are recorded as text-color masks in
`fixtures/sweets-menu-screens.json`, without background artwork. The diagnostic
`python3 tools/analyze_menu_samples.py` checks whether each expected character
has an exact isolated shape somewhere in its corresponding screen; this is
only a font-coverage check, not evidence of correct segmentation, ordering,
false-positive rejection, different sizes, or different firmware versions.

The experiment exposed wrong small-kana indices in the prior table: codes
042F–0437 render ぁぃぅぇぉっゃゅょ, and 045B renders a middle dot. The
memory decoder and local translations were corrected accordingly. The positioned samples are now used by the app's exact 12-pixel matcher.
It scans the frame for text runs without fixed row coordinates. Tests include
main menus, submenus and non-menu scenes. Some unknown glyphs cause adjacent
fragments to be suppressed, although this does not cover every possible layout.
The matcher also covers gray disabled entries, digits, uppercase Latin letters,
and the proportional m!x mark. Tests include several additional Sweets
submenus. The status page also draws a smaller font. A snapshot paused after the first
name character was drawn but before the second was read allowed a second
character to be substituted and captured with `capture_small_font.py`. The
resulting 276 samples cover a 10x9 area because voiced marks extend beyond
the nominal 8-pixel glyph. The matcher allows this overlap and uses the local
translation dictionary only when an ambiguous whole word has one match.
Independent status frames with かの, かい and かだ exercise both sizes. A
room and other non-text frames serve as negative samples. This is not a broad
false-positive or complete character-coverage proof.

The current Sweets session uses the new matcher; other ROM versions keep the
previous menu path. A synthetic unknown glyph in a Sweets screen is retained as an
explicit partial-recognition regression: the adjacent fragment is suppressed.
This check does not cover all possible unknown-glyph placements.

The status details fixture exercises generation, age and Gotchi Points,
including a synthetic change to two-digit generation and age. Profile date
units, a clipped small-font line, the white-on-cyan vertical encyclopedia
label, and the clock's distinct date/time fonts are also covered by frames
from Sweets. Clock samples include AM/PM and blinking fields. These are
specific verified cases: other kanji, layouts, font sizes, vocabulary,
unrecognized code pages, and non-menu screens remain to be investigated.
A recognized fragment may still be emitted when a neighboring glyph is unknown.

The full-frame corpus currently contains 93 examples, including synthetic
variations of real captures. `inspect_sweets_lcd` can additionally run a
user-supplied snapshot and decode the emulator framebuffer directly; it does
not require an iOS simulator or store the firmware or save data.

Fifteen additional pictograms (04D0–04DE) were captured in both sizes. Their
Unicode labels are approximate visual substitutes for the game graphics, not
official names. Mixed text and icon runs were also checked by rendering each
code through the firmware and decoding the resulting live framebuffer.

Two-character 12-pixel runs no longer require a dictionary entry: an
independently rendered `きぬ` example verifies names outside the translation
table. The 65 full-frame fixtures and 43 local emulator snapshots were checked
for changed output; these samples cannot rule out all false positives.

A status card with a single-character small-font name is also recognized when
it sits immediately above an independently recognized larger species name.
A real frame and a substituted second character exercise this contextual rule.

Small-font matching compares the left overhang exactly while permitting ink
in the rightmost column from the following character. This distinguishes
コ/ロ and J/U without a dictionary; four firmware-rendered names cover these
cases, and a fixture invariant checks the captured glyphs' rightmost column.

`check_sweets_font_context.py` checks captured code points in firmware-rendered
mixed runs using caller-provided snapshots and the live Rust decoder. This
checks the supplied contexts, not every possible glyph adjacency or layout.
A dense 8px gift pictogram initially failed the background threshold; a real
frame now covers that case.
The dense gift plus a following left-overhanging glyph is covered separately;
an unrecognized following shape must not be reported as a complete one-letter
status name.
Small-font runs of multiple characters also reject adjacent unexplained dark
ink on a light background. The tests include a corrupted final character;
this remains a heuristic for other backgrounds and decorations.

The matcher also supports a 16px-wide nearest-neighbour 4:3 enlargement
of the captured 12px font. Its tests use synthetic scaled glyphs, including
an unknown adjacent shape; no Sweets screen has yet established whether the
game uses this rendering for its larger text. Other 16px designs still need
independent capture and validation.
Scaled runs can also be joined across small gaps or a blank cell; their
actual pixel bounds, rather than the number of output characters, determine
overlap with smaller text.

Additional code pages 10, 18, 1C and 20 contain accented Latin glyphs.
Thirty-one code points were captured at both sizes and independently checked
in firmware-rendered mixed runs; some codes produce identical glyphs, which
the matcher deduplicates. Other code pages have not been exhaustively audited.

The message-buffer decoder accepts the explicitly mapped extended Latin codes
alongside page-04 kana. Unmapped non-kana pages still terminate a run. A
sampling survey found 0801–0820 blank, page 05 initial kana identical to 04,
and sampled high offsets in pages 10/18/1C/20 identical to page-04 glyphs;
this does not establish the meaning of every unsampled control code.

A real speech bubble containing a single 8px あ is recognized using the
bubble's horizontal borders as context. Its former empty annotation was
corrected after inspecting the original emulator frame.

Small-font runs preserve one or two blank cells when another recognized glyph
follows on the same baseline. This is tested by adding captured glyphs to a
real speech-bubble frame; other spacing and layouts remain unverified.

Clock settings retain the visible part when the selected month, day, hour
or minute is blinked off. Four real frames exercise these partial values;
the missing field is left blank rather than inferred.

The 12px and scaled 16px scans cache rolling horizontal pixel windows to
avoid repeatedly reading overlapping cells. A direct-pixel equivalence test
and the full-frame corpus cover the optimization. On the available x86_64
release build, 73 frames took about 0.47 seconds; this is not an iOS benchmark.

A family card uses small 代目 and 才 bitmaps on white or pale-yellow
backgrounds. Its generation number is above the unit rather than inline; a
real frame and substituted digits cover generation and age. Two additional
real warning screens cover multi-line menu text.

Further emulator exploration covers the item-exchange destination menu, a
camera prompt mixing Japanese and Latin text, and two additional encyclopedia
entries with vertical labels and different names.

Additional communication screens cover three- and four-line disabled menus
and the proportional m!x mark after a different Japanese word.

Nearby vertical columns with aligned tops are assembled right-to-left. This
ordering is tested with two different columns derived from a real sidebar;
a native multi-column Sweets screen has not yet been captured.

A fresh-game name keyboard supplies mixed-size examples, a pictogram-only
selection row, and adjacent green name-entry boxes. Pictogram-only runs are
accepted with nearby pictogram context; a narrow white rule between green
boxes is treated as a visual separator rather than a text space.

A fresh save was advanced through date setup and name confirmation. Real
frames include a seven-character name and the three-line confirmation page;
the latter has matching local translations.

The remaining main-page code 04FD is a second encoding of ー in both sizes
and is now included. 046C renders as a blank cell; 04C8–04CF and 04FF
rendered blank in this menu probe. The latter may be a control, so these
observations do not establish their behavior in every context.

The naming keyboard's number, Latin and single return-symbol pages are
covered by real frames. The isolated return glyph requires the keyboard
background context to avoid treating arbitrary single symbols as text.

## 16px verification and false positives

The synthetic 16px path is now exercised against every frame in
`sweets-full-screens.json`. None of the 93 real Sweets frames produces a
complete 16px line; `scaled_lines` stays empty for all of them. One frame,
`ツーしん`, contains a single 16px-sized cell from the wide ー glyph, but the
matcher requires at least two adjacent cells and therefore does not emit it.
The regression test rejects more than one stray 16px cell per frame so a new
false positive is surfaced instead of silently accepted.

There is still no real Sweets screen in the corpus that establishes a
16px line. The 16px path remains a synthetic enlargement of the 12px font;
using it on a real large-text screen would require an independently captured
frame.

Current verification:

- 93 full frames, including four negative (non-text) frames, all expected
  outputs match.
- 277 12px glyph samples and 276 8px samples.
- 8px/12px context check: 0 mismatches.
- 61 Rust tests and 9 Python tool tests pass.

These are explicit corpus checks, not an exhaustive proof. Unknown code pages,
unknown glyph placements, other layouts and other firmware versions remain
outside the verified set.

## Proportional glyphs in a gap

`menu_text::read` matches a 12-pixel window, which only works when the next
glyph starts at least 12 pixels later. A narrow glyph such as ょ (9 px) in the
tight 12-pixel advance of the birthday screen never forms a clean window, so
the strict pass dropped both halves of the line.

The line joiner now decodes the ink between two matched runs against trimmed
glyph shapes (`trimmed()` in `menu_text.rs`) before giving up. On the captured
「あなたの たんじょうび」frame this recovers ょ and yields the full line.
`tools/fixtures/large-text-screens.json` is a real 128x128 frame (white text
panel, dark blue 12px font) that covers this layout.

The 14-px date line on the same screen (`1／1`) is a different, larger font and
is still not read; only its first `1` matches the 12px table, and the
two-character minimum keeps it from being emitted.

## Name entry and setup screens

The name-entry keyboard draws its kana list on a solid pink bar. Those glyphs
are selection UI, not message text, so `read` drops any line whose background
is mostly that pink (`on_kana_bar`). The prompt above the keyboard is kept, and
the corpus labels for the keyboard frames were reduced to the prompt.

Two whole-screen rules live in the app, not the matcher:

- The date/time setup value (`1/1 AM 12:00`, including the blinking field) is
  not message text, so the panel shows the placeholder instead.
- The name prompt is pre-written into the message buffer while the egg and
  birthday scenes are on screen. The panel only shows it when the glyph matcher
  reads it from the screen, so it no longer appears before the screen asks.
