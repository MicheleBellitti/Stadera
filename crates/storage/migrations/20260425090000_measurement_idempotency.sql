-- Add UNIQUE constraint on measurements for idempotent inserts during Withings sync.
--
-- Without this, retrying a sync after a partial failure (or after Withings
-- returns an overlapping window of measurements on a subsequent poll) would
-- create duplicate rows: each insert generates a fresh UUID v7 PK, so PK
-- uniqueness is not a guard.
--
-- (user_id, taken_at, source) is the natural business key: a single user
-- never has two measurements at the exact same instant from the same source.
-- Manual entries that happen to land on the same second as a Withings reading
-- are still distinguishable thanks to `source`.

ALTER TABLE measurements
    ADD CONSTRAINT measurements_user_taken_at_source_key
    UNIQUE (user_id, taken_at, source);
