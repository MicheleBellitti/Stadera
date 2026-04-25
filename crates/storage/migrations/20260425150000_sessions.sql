-- Server-side session storage for the HTTP API.
--
-- The session ID is a UUID v7 (time-ordered) carried in an HttpOnly cookie.
-- The user_id FK has ON DELETE CASCADE so deleting a user wipes their sessions.
--
-- An index on `expires_at` supports a future cleanup job that deletes
-- expired rows. An index on `user_id` supports "log out all my devices".

CREATE TABLE sessions (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_id_idx ON sessions(user_id);
CREATE INDEX sessions_expires_at_idx ON sessions(expires_at);
