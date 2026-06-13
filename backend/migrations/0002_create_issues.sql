CREATE TYPE issue_priority AS ENUM ('P1', 'P2', 'P3', 'P4');
CREATE TYPE issue_status   AS ENUM ('Open', 'InProgress', 'Resolved', 'Closed');

-- Bump updated_at on every UPDATE. Shared by issues and comments.
CREATE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE issues (
    id          BIGSERIAL      PRIMARY KEY,
    title       TEXT           NOT NULL,
    description TEXT           NOT NULL DEFAULT '',
    priority    issue_priority NOT NULL DEFAULT 'P3',
    author      TEXT           NOT NULL,
    status      issue_status   NOT NULL DEFAULT 'Open',
    created_at  TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE TRIGGER issues_set_updated_at
    BEFORE UPDATE ON issues
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX issues_status_idx   ON issues (status);
CREATE INDEX issues_priority_idx ON issues (priority);

CREATE TABLE issue_assignees (
    issue_id  BIGINT NOT NULL REFERENCES issues (id) ON DELETE CASCADE,
    author_id TEXT   NOT NULL,
    PRIMARY KEY (issue_id, author_id)
);

-- The Map<key, {guild, channel, message}> from ISSUE.md. key e.g. 'original_reference', 'issue'.
CREATE TABLE issue_discord_messages (
    issue_id   BIGINT NOT NULL REFERENCES issues (id) ON DELETE CASCADE,
    key        TEXT   NOT NULL,
    guild_id   BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    PRIMARY KEY (issue_id, key)
);

CREATE TABLE comments (
    id         BIGSERIAL   PRIMARY KEY,
    issue_id   BIGINT      NOT NULL REFERENCES issues (id) ON DELETE CASCADE,
    author     TEXT        NOT NULL,
    content    TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER comments_set_updated_at
    BEFORE UPDATE ON comments
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE INDEX comments_issue_id_idx ON comments (issue_id);
