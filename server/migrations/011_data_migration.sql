-- The steps that change the data rather than the schema, each run once by the
-- server (src/dataMigrations.js), and what each did.
CREATE TABLE data_migration (
  name       TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  note       TEXT NOT NULL DEFAULT ''
);
