CREATE TYPE summary_status AS ENUM ('Pending', 'Confirmed', 'Discarded');

-- AI-summary conversation: one draft per temp channel, refined with the dev before becoming an issue.
CREATE TABLE summary_drafts (
    id              BIGSERIAL      PRIMARY KEY,
    temp_channel_id BIGINT         NOT NULL,
    author_id       TEXT           NOT NULL,
    priority        issue_priority,
    summary         TEXT,
    status          summary_status NOT NULL DEFAULT 'Pending',
    issue_id        BIGINT         REFERENCES issues (id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ    NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE TRIGGER summary_drafts_set_updated_at
    BEFORE UPDATE ON summary_drafts
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE summary_draft_messages (
    id         BIGSERIAL   PRIMARY KEY,
    draft_id   BIGINT      NOT NULL REFERENCES summary_drafts (id) ON DELETE CASCADE,
    role       TEXT        NOT NULL,
    content    TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX summary_draft_messages_draft_id_idx ON summary_draft_messages (draft_id);
