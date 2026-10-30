# signalling — what the signalling domain may hold

`packages/core/src/signalling/` · needs `track`

The content of the domain `signalling` 1.0 ([schema](../../../schemas/signalling/1.0.schema.json),
[decision 0011](../../../docs/decisions/0011-signalling-domain.md)) that its schema leaves open: which signal
aspects there are, which signalling systems and which kind of signal shows what in them, which train
protection systems with which balises and magnets and their settings — the catalogue of DB Ril 301 after
PlanPro 1.10: Ks and shunting signals, PZB and ETCS ([`constraints/db-ril-301.json`](../constraints/db-ril-301.json))
— and the check of what a schema cannot say: references, locations on their tracks, what each signal shows,
routes, overlaps, flank protection, balises and station boundaries.

The schema gives the structure, after PlanPro 1.10; the catalogue gives the content, as PlanPro splits
`Signalbegriffe_Struktur.xsd` from `Signalbegriffe_Ril_301.xsd` and OSRD splits RailJSON from its signalling
systems. As in OSRD, a signal at the track carries a **logical signal** per system it shows — its system, its
kind there and the system's settings —, and a **balise** names its train protection system, its kind and its
settings: a new signalling system (H/V, Sv) is a new entry under `systeme`, a new train protection system
(ZUB, LZB's loop ends) one under `zugbeeinflussung.systeme`, never a new schema. Every entry has a German
`title` and an English `title_en`; `titleIn(entry, language)` gives the one a tool shows.

## Files

| File | What it does |
|---|---|
| `catalogue.js` | The catalogue: an aspect by its key (`aspectOf`), what a kind of signal of a system shows (`signalKindOf`) and what a screen of several logical signals shows (`screenOf`), a train protection system and its kinds (`protectionSystemOf`, `baliseKindOf`), how settings stand to what is declared (`settingsProblems`), whether a symbol fits, which aspects stand on a board of their own, an entry's title in a language (`titleIn`) |
| `checkSignalling.js` | `checkSignalling({ core, signalling })`: the findings of the domain on the core it stands on, `{ errors, warnings }` as validateProject gives them |
| `trackSections.js` | The extent of the track detection sections, derived over the net from their points (`trackSectionExtents`, `netParts`) |

## The catalogue

| Part | What it says |
|---|---|
| `begriffe` | all 192 aspects of `Signalbegriffe_Ril_301.xsd` by their key — PlanPro's name (`Hp_0`, `Ks_1`, `Zs_3` …) — with name and title, the frames it may stand on (`screen`, `indicator`, `plate`, `body`; none named: any), the symbols it takes (`symbole`, from the XSD's enumeration; `symbol_frei` for free text; `symbol_bedeutung: speed` where they are tens of km/h), whether it can be lit and switched |
| `systeme` | the signalling systems a logical signal is of: `ks` with its kinds (`main`, `mainShunting`, `multiSection`, `multiSectionShunting`, `shunting`, `distant`, `distantRepeater`), `sh`, the shunting signal (Sh 0, Sh 1). Per kind what its screen must show (`pflicht`), at least one of (`fahrt`), and may show (`erlaubt`) — Kl, Zl, Zs 1, Zs 7 and Zs 8 are lights of the Ks screen, as PlanPro plans have them — and which indicators it carries (`zusatz`); a system may declare `einstellungen` for its logical signals. A signal of several logical signals shows what they ask together |
| `ohne_system` | the aspects that may stand on a board of their own (`tafeln`: Ne, Lf, Ra, El, So, Bü, Oz …, not the mast plates Ms) |
| `zugbeeinflussung` | the train protection systems of balises and magnets — `pzb` (track magnet, speed check), `etcs` (fixed and controlled Eurobalise) —, each with its short `name`, the settings it declares (`einstellungen`: a choice of `werte`, a number with `einheit`, `min`, `max`, a flag, a text; `vorgabe` the value a tool starts with) and its kinds (`arten`), each with the settings it must (`pflicht`) and may (`erlaubt`) have; how a balise acts (`wirksamkeit`), with PlanPro's names |
| `open_points` | what is not in yet: distant signal spacing, overlap lengths, where balises and magnets lie (Ril 819, Ril 483) |

## The check

| Finding | Severity | When |
|---|---|---|
| `reference_missing` | error | a reference finds nothing: in the domain, in the core (tracks, switches, operational points, stages) |
| `off_track`, `orphaned` | error, warning | a station beyond its track; a location in the plane, without a track (R7) |
| `aspect_unknown`, `aspect_not_on_frame`, `symbol_invalid` | error | an aspect the catalogue does not have, on a frame it does not belong on, without the symbol it asks for |
| `frame_empty` | warning | a frame that shows nothing, other than a name plate |
| `screen_lacks`, `screen_not_allowed`, `screens_many` | error | a screen without an aspect its kind must show, with one it may not, more than one screen |
| `screen_no_proceed` | warning | a screen showing none of a kind's proceed aspects (an exit signal of P-Hausen shows Hp 0 and Sh 1 only) |
| `indicator_not_allowed`, `board_not_allowed` | error | an indicator the kind does not carry; a board on its own that is none |
| `system_unknown`, `system_twice`, `type_missing`, `type_not_in_system` | error | each logical signal's system and kind, each system once; a screen whose logical signals name no kind. A screen of no logical signal (a level crossing's monitoring signal, a lone Zs 6) is checked for known aspects only |
| `setting_missing`, `setting_unknown`, `setting_invalid` | error | the settings of a logical signal or a balise: what its kind asks for, nothing its system and kind do not declare, each value as declared |
| `virtual_with_frames`, `virtual_without_function` | error, warning | a virtual signal shows nothing and says what it stands for |
| `route_start_off_path`, `route_target_off_path`, `route_to_itself`, `path_empty` | error | a route's run of tracks and its two signals |
| `route_without_overlap`, `shunting_with_overlap`, `overlap_preferred_many`, `overlap_not_at_target` | warning, error, error, error | a train route to a real signal ends in an overlap behind it, one preferred at most; a shunting route has none |
| `route_aspect_not_on_signal`, `route_type_missing` | error, warning | a route shows what its signal has, and says its kind |
| `overlap_start_off_path`, `overlap_end_off_path` | error | an overlap runs from its signal's track; its end, the danger point, lies on its run or beside its end (the clearance marker of a switch there) |
| `flank_asked_twice`, `flank_by_itself`, `flank_position_missing` | error | one flank protection per switch element and position, never given by the element itself; a switch giving it states its position (a derailer gives it applied) |
| `system_unknown`, `kind_not_in_system`, `balise_switched_without_signal` | error | a balise of a train protection system and kind the catalogue knows; one its signal switches names it |
| `detection_point_unused` | warning | a detection point that bounds no track section |
| `section_extent_differs` | warning | a section's points enclose no part of the net together (see below) |
| `boundary_mark_special` | warning | a station boundary marked by neither an entry signal nor a board Ne 1 nor a switch — a special case of EBO §4 |

## Track sections over the net

A section is stored with its name, kind and the points that bound it; what it covers is derived. Its own
points cut the net into parts — runs of track between them and the track ends, joined across switches, links
and joints — and it covers the part bounded by all of them. Only its own: sections overlap where two meet,
each ending at a counting head of its own a metre or so beyond the other's. Where several parts are bounded
by all its points (a section reaching to a track end), the one holding fewest points of other sections is it;
where none is, `section_extent_differs` names the points that enclose nothing with the others.

On P-Hausen (decision 0011), 44 of the 56 sections come out as PlanPro's Gleis_Abschnitt states them, 7 differ
by their overlap at the ends, and the two 700 m track circuits come out at the length their FMA_Anlage states
(PlanPro's Gleis_Abschnitt gives 297 m).

Not checked yet: distances (catalogue `open_points`).

## In the record

The Layout-Tool's record carries the domain whole, as `signalling: { fields, collections }`
([document](document.md)): it does not edit it, but writes it back as it came. Carrying its locations when a
track is split, joined, reversed or deleted is the core's (R4, [merge](merge.md)).
