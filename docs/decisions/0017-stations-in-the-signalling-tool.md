# 0017 — The Signalling-Tool creates the stations, in the core's register

Status: **accepted** · 2026-11-02 · amends R1 of the [data model](../data-model.md) and
[0011](0011-signalling-domain.md)

## Decision

1. **A station is created in the Signalling-Tool**, under *Bahnhöfe*, above the switches in its rail. A station
   (EBO §4) is a railway facility with at least one switch where trains may start, end, pass or turn back. The
   station is a point of the core's operational point register (R10, F6): its name and, once there is one, its
   RL100 code. The Signalling-Tool writes it there. This is an exception to R1, as the core library's own
   changes are.
2. **The register holds what the record keeps of it.** The record (oltjson-1.1) gains `operationalPoints`, the
   points the tracks' and platforms' station names do not make by themselves:
   - a station that nothing names yet;
   - codes that no object states;
   - the id of a station that was renamed, or that the signalling names.

   A kept point is the one of its name. Tracks and platforms that name it are of it. One that states another
   code is a point of its own, as two codes under one name always were. A record whose names make every point
   keeps none, so the documents written before stay as they were.
3. **A platform is assigned to its station in the Signalling-Tool** by the station's name and code on the
   platform (`stationName`, `code`). The Layout-Tool sets the same fields; the two tools write one fact in one
   place.
4. **A signal belongs to a station or to the open line**: `operationalPointId` on the signal (signalling 1.0,
   changed in place a fourth time; the door reads a domain of the schema before as of the open line). Its
   editor names the station; the station's editor lists its signals.
5. **The boundary towards the open line** stays the signalling's (`stationBoundaries`). In general it is the
   station's entry signals or trapezoid boards (Ne 1), otherwise its entry switches. The station's editor
   proposes the entry signals and boards that belong to it. A boundary marked by a signal of the open line or
   of another station is a warning (`boundary_mark_elsewhere`).
6. **The Layout-Tool stays the owner of the station tracks.** A station that tracks name is renamed and deleted
   there. The Signalling-Tool gives such a station its code, signals, platforms and boundary.

## Why

The identity of a station is one fact with one owner (R10). Before this, only station texts on tracks and
platforms in the Layout-Tool made it. A signal engineer who plans a station's signals had no way to name a
station that no track carried yet: a new halt, or a station on an open-line track. Holding a second list of
stations in the signalling domain would make the same fact twice. So the Signalling-Tool writes the register
itself, and the register can hold points of its own.

A signal names its station by the register's id. Station texts can change in the Layout-Tool, so the record
keeps every point the signalling names. Its id then outlives a renaming of the tracks, and the signalling's
references never dangle because of a rename elsewhere.

## What it costs

- **R1 has a second exception.** Two tools write the core: the Layout-Tool everything, the Signalling-Tool the
  register's points and a platform's station. A merge compares both field by field (`operationalPoints` is a
  collection of the merge); a conflict in them is decided in the Layout-Tool, as every conflict in the core.
- **The register is drawn and kept at once.** What the names make and what the record keeps are combined at
  the door; a record keeps only what the names do not make. The rule is the core's (`coreFormat.js`) and tested
  there, both ways.
- **Two names for a station can stand side by side.** A station the signalling names keeps its name when its
  tracks are named anew in the Layout-Tool; the tracks then make a station of their own. The Signalling-Tool
  shows both, and the station can be renamed there to the tracks' name, which joins them again.
- **The inner switches are not checked.** That a station has at least one switch is its definition; the check
  does not derive which switches lie within its boundary yet.
