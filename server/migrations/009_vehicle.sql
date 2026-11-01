-- The suite's vehicles, for the Fahrplan-Sim (decision 0014): a catalogue
-- for everyone signed in, beside the projects, not in a project's record. A
-- vehicle is kept as OSRD's RailJSON rolling stock (data, JSON) — its
-- physics, its tractive effort curves, its metadata —, so that it goes to
-- OSRD and comes from it as it is. version counts the saves: a save names
-- the version it changed, and one on an older version is refused.
CREATE TABLE vehicle (
  id          TEXT    PRIMARY KEY,
  name        TEXT    NOT NULL,
  data        TEXT    NOT NULL,
  version     INTEGER NOT NULL DEFAULT 1,
  created_by  INTEGER NOT NULL REFERENCES user(id),
  created_at  TEXT    NOT NULL,
  updated_by  INTEGER NOT NULL REFERENCES user(id),
  updated_at  TEXT    NOT NULL,
  deleted_at  TEXT
);
CREATE INDEX vehicle_name ON vehicle(name);
