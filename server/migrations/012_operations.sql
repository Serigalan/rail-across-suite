-- A project's operations in revisions (operations-1.0; the roadmap, phase 4,
-- O2; decision 0026): each revision the whole operations document of the
-- project — its timetables and their trains, the kinds of trains, the
-- formations, … — immutable, numbered per project, by hash, as a variant's
-- states are. A timetable saved makes a new revision; `changed` names the
-- timetables it changed. The rows of `timetable` (migration 010) became each
-- project's first revision (data step 0026-operations) and are read no more.
CREATE TABLE operations_revision (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id  TEXT    NOT NULL REFERENCES project(id),
  number      INTEGER NOT NULL,
  hash        TEXT    NOT NULL,
  payload     BLOB    NOT NULL,
  changed     TEXT    NOT NULL DEFAULT '[]',
  created_by  INTEGER NOT NULL REFERENCES user(id),
  created_at  TEXT    NOT NULL,
  UNIQUE (project_id, number)
);

-- Who made a timetable and who saved it last, and when: what the revisions say
-- of it, kept beside them so a list needs no walk through them.
CREATE TABLE operations_timetable (
  project_id   TEXT    NOT NULL REFERENCES project(id),
  timetable_id TEXT    NOT NULL,
  created_by   INTEGER NOT NULL REFERENCES user(id),
  created_at   TEXT    NOT NULL,
  updated_by   INTEGER NOT NULL REFERENCES user(id),
  updated_at   TEXT    NOT NULL,
  PRIMARY KEY (project_id, timetable_id)
);
