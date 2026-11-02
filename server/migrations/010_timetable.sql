-- The timetables of a project, for the Fahrplan-Editor (decision 0016): its
-- trains — their numbers, vehicles, departures, the stations they start at,
-- stop at, pass and end at — as JSON (data), beside the project's variants,
-- not in a revision: a timetable is laid on any variant of its project, the
-- stations named by their names. Seen and changed by whoever sees the
-- project; a template's by the admins alone. version counts the saves: a
-- save names the version it changed, and one on an older version is refused.
CREATE TABLE timetable (
  id          TEXT    PRIMARY KEY,
  project_id  TEXT    NOT NULL REFERENCES project(id),
  name        TEXT    NOT NULL,
  data        TEXT    NOT NULL,
  version     INTEGER NOT NULL DEFAULT 1,
  created_by  INTEGER NOT NULL REFERENCES user(id),
  created_at  TEXT    NOT NULL,
  updated_by  INTEGER NOT NULL REFERENCES user(id),
  updated_at  TEXT    NOT NULL,
  deleted_at  TEXT
);
CREATE INDEX timetable_project ON timetable(project_id);
