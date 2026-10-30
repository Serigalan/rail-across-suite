# signalling — what the signalling domain may hold

`packages/core/src/signalling/` · needs nothing

The content of the domain `signalling` 1.0 ([schema](../../../schemas/signalling/1.0.schema.json),
[decision 0011](../../../docs/decisions/0011-signalling-domain.md)) that its schema leaves open: which signal
aspects there are and which a kind of signal shows — the catalogue of DB Ril 301, Ks with PZB
([`constraints/db-ril-301.json`](../constraints/db-ril-301.json)) — and the check of what a schema cannot say:
references, locations on their tracks, routes, overlaps, flank protection, PZB and station boundaries.

The schema gives the structure, after PlanPro 1.10; the catalogue gives the content, as PlanPro splits
`Signalbegriffe_Struktur.xsd` from `Signalbegriffe_Ril_301.xsd` and OSRD splits RailJSON from its signalling
systems. A new system (H/V, Sv) is a new entry under `systeme` in the catalogue, not a new schema.

## Files

| File | What it does |
|---|---|
| `catalogue.js` | The catalogue: an aspect by its key (`aspectOf`), what a kind of signal of a system shows (`signalKindOf`), whether a symbol fits, which aspects stand on a board of their own |
| `checkSignalling.js` | `checkSignalling({ core, signalling })`: the findings of the domain on the core it stands on, `{ errors, warnings }` as validateProject gives them |

## The catalogue

| Part | What it says |
|---|---|
| `begriffe` | every aspect by its key — PlanPro's name (`Hp_0`, `Ks_1`, `Zs_3` …) — with its name, its title as PlanPro gives it, the frames it may stand on (`screen`, `indicator`, `plate`) and the symbol it asks for (`speed`: tens of km/h, 1–16; `letter`) |
| `systeme.ks.signalarten` | per kind of Ks signal (`main`, `mainShunting`, `multiSection`, `multiSectionShunting`, `distant`, `distantRepeater`) what its screen must and may show, and which indicators it carries |
| `ohne_system` | the shunting signal (Sh 0, Sh 1), and the aspects that stand on a board of their own (Ne 1, Ne 2, Ne 4, Ne 5, Lf 6, Lf 7, Ra 10, Zs 3, Zs 3v) |
| `pzb` | the frequencies of the track elements and how they act, with PlanPro's names |
| `open_points` | what is not in yet: distant signal spacing, overlap lengths, where the PZB magnets lie (Ril 819, Ril 483) |

## The check

| Finding | Severity | When |
|---|---|---|
| `reference_missing` | error | a reference finds nothing: in the domain, in the core (tracks, switches, operational points, stages) |
| `off_track`, `orphaned` | error, warning | a station beyond its track; a location in the plane, without a track (R7) |
| `aspect_unknown`, `aspect_not_on_frame`, `symbol_invalid` | error | an aspect the catalogue does not have, on a frame it does not belong on, without the symbol it asks for |
| `screen_lacks`, `screen_not_allowed`, `screens_many` | error | a screen without an aspect its kind must show, with one it may not, more than one screen |
| `indicator_not_allowed`, `board_not_allowed` | error | an indicator the kind does not carry; a board on its own that is none |
| `system_unknown`, `type_missing`, `type_not_in_system` | error | the screen's system and kind |
| `virtual_with_frames`, `virtual_without_function` | error, warning | a virtual signal shows nothing and says what it stands for |
| `route_start_off_path`, `route_target_off_path`, `route_to_itself`, `path_empty` | error | a route's run of tracks and its two signals |
| `route_without_overlap`, `shunting_with_overlap`, `overlap_preferred_many`, `overlap_not_at_target` | warning, error, error, error | a train route to a real signal ends in an overlap behind it, one preferred at most; a shunting route has none |
| `route_aspect_not_on_signal`, `route_type_missing` | error, warning | a route shows what its signal has, and says its kind |
| `overlap_start_off_path`, `overlap_end_off_path` | error | an overlap runs from its signal's track and ends on its last track |
| `flank_asked_twice`, `flank_by_itself` | error | one flank protection per switch element and position, never given by the element itself |
| `speed_check_values_missing`, `magnet_with_speed_check`, `pzb_switched_without_signal` | error | a PZB speed check and a magnet as their kinds are |
| `detection_point_unused` | warning | a detection point that bounds no track section |
| `boundary_mark_special` | warning | a station boundary marked by neither an entry signal nor a board Ne 1 nor a switch — a special case of EBO §4 |

Not checked yet: distances (catalogue `open_points`), and the extent of a track section derived over the net
from its detection points, which needs the topology.

## In the record

The Layout-Tool's record carries the domain whole, as `signalling: { fields, collections }`
([document](document.md)): it does not edit it, but writes it back as it came. Carrying its references when a
track is split, joined or reversed (R4) comes with the signalling tool.
