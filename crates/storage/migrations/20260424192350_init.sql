-- Add migration script here
-- Initial schema: users, user_profiles, measurements, withings_credentials.
-- Multi-tenant-ready from day 1 (every table has user_id).

CREATE TABLE users (
    id         UUID PRIMARY KEY,
    email      VARCHAR(320) NOT NULL UNIQUE,
    name       VARCHAR(200) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_profiles (
    user_id        UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    sex            VARCHAR(10) NOT NULL CHECK (sex IN ('male', 'female')),
    birth_date     DATE NOT NULL,
    height_cm      DOUBLE PRECISION NOT NULL CHECK (height_cm BETWEEN 50 AND 300),
    activity_level VARCHAR(20) NOT NULL CHECK (activity_level IN ('sedentary', 'lightly_active', 'moderately_active', 'very_active')),
    goal_weight_kg DOUBLE PRECISION NOT NULL CHECK (goal_weight_kg BETWEEN 10 AND 500),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE measurements (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    taken_at         TIMESTAMPTZ NOT NULL,
    weight_kg        DOUBLE PRECISION NOT NULL CHECK (weight_kg BETWEEN 10 AND 500),
    body_fat_percent DOUBLE PRECISION CHECK (body_fat_percent BETWEEN 2 AND 80),
    lean_mass_kg     DOUBLE PRECISION CHECK (lean_mass_kg BETWEEN 2 AND 300),
    source           VARCHAR(20) NOT NULL CHECK (source IN ('withings', 'manual')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX measurements_user_taken_at_idx ON measurements (user_id, taken_at DESC);

CREATE TABLE withings_credentials (
    user_id           UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    access_token_enc  BYTEA NOT NULL,
    refresh_token_enc BYTEA NOT NULL,
    expires_at        TIMESTAMPTZ NOT NULL,
    scope             VARCHAR(500) NOT NULL,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);