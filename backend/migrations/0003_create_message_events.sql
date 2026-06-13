-- Mirror of Discord messages, fed by the bot's message create/edit/delete events.
CREATE TABLE message_events (
    message_id BIGINT      PRIMARY KEY,
    guild_id   BIGINT,
    channel_id BIGINT      NOT NULL,
    author_id  TEXT,
    content    TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    edited_at  TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);
