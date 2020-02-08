ALTER TABLE messages RENAME TO _messages_old;

CREATE TABLE messages (
	id INTEGER NOT NULL PRIMARY KEY,
	chat INTEGER NOT NULL REFERENCES chats(id),
	-- NULL if we got a message from the server
	invoker BLOB REFERENCES clients(uid),
	-- The name if the uid is not known
	invoker_name TEXT,
	content TEXT NOT NULL,
	-- In UTC
	time DATETIME NOT NULL,
	-- Offset from UTC in seconds to the east
	timezone INTEGER NOT NULL
);

INSERT INTO messages (id, chat, invoker, invoker_name, content, time, timezone)
	SELECT id, chat, invoker, invoker_name, content, time, timezone
	FROM _messages_old;

DROP TABLE _messages_old;
CREATE INDEX messages_chat_index ON messages(chat);
CREATE INDEX messages_client_index ON messages(invoker);
