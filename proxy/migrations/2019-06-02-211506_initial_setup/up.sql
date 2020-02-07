PRAGMA foreign_keys = ON;

CREATE TABLE clients (
	-- sha1(base64(primary key))
	uid BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	public_key BLOB,
	custom_name TEXT
);

CREATE TABLE identities (
	id INTEGER NOT NULL PRIMARY KEY,
	-- encrypted
	private_key BLOB NOT NULL,
	name TEXT NOT NULL DEFAULT "Default",
	counter INTEGER NOT NULL DEFAULT 0,
	max_counter INTEGER NOT NULL DEFAULT 0,
	client BLOB NOT NULL REFERENCES clients(uid)
);
CREATE INDEX identities_client_index ON identities(client);

CREATE TABLE servers (
	public_key BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	-- Last used address
	address TEXT NOT NULL,
	icon INTEGER
);

CREATE TABLE channels (
	server BLOB NOT NULL REFERENCES servers(public_key),
	id INTEGER NOT NULL,
	parent INTEGER,
	-- References the channel above this one (zero if the first)
	order_id INTEGER,
	name TEXT NOT NULL,
	icon INTEGER,
	deleted BOOLEAN NOT NULL DEFAULT false,

	PRIMARY KEY(server, id),
	FOREIGN KEY(server, parent) REFERENCES channels(server, id)
);
CREATE INDEX channels_server_index ON channels(server);
CREATE INDEX channels_parent_index ON channels(server, parent);

CREATE TABLE bookmarks (
	id INTEGER NOT NULL PRIMARY KEY,
	name TEXT,
	username TEXT NOT NULL,
	address TEXT NOT NULL,
	channel INTEGER,
	identity INTEGER NOT NULL REFERENCES identities(id),
	bookmark BOOLEAN NOT NULL DEFAULT false,
	-- In UTC
	last_used DATETIME,
	-- Offset from UTC in seconds to the east
	timezone INTEGER NOT NULL,
	-- References the server if already connected once
	server BLOB REFERENCES servers(public_key),

	FOREIGN KEY(server, channel) REFERENCES channels(server, id)
);
CREATE INDEX bookmarks_identity_index ON bookmarks(identity);
CREATE INDEX bookmarks_server_index ON bookmarks(server);
CREATE INDEX bookmarks_channel_index ON bookmarks(server, channel);

CREATE TABLE chats (
	id INTEGER NOT NULL PRIMARY KEY,
	-- In UTC
	last_read DATETIME NOT NULL,
	-- Offset from UTC in seconds to the east
	timezone INTEGER NOT NULL
);

CREATE TABLE messages (
	id INTEGER NOT NULL PRIMARY KEY,
	chat INTEGER NOT NULL REFERENCES chats(id),
	-- NULL if we got a message from the server
	invoker BLOB REFERENCES clients(uid),
	-- The name if the uid is not known
	invoker_name TEXT,
	content TEXT NOT NULL,
	status TEXT CHECK(status IN ('sending', 'success', 'error')) NOT NULL DEFAULT 'success',
	-- In UTC
	time DATETIME NOT NULL,
	-- Offset from UTC in seconds to the east
	timezone INTEGER NOT NULL
);
CREATE INDEX messages_chat_index ON messages(chat);
CREATE INDEX messages_client_index ON messages(invoker);

CREATE TABLE events (
	id INTEGER NOT NULL PRIMARY KEY,
	server BLOB REFERENCES servers(public_key),
	invoker BLOB REFERENCES clients(uid),
	channel1 INTEGER NOT NULL,
	channel2 INTEGER NOT NULL,
	client BLOB REFERENCES clients(uid),
	typ TEXT CHECK(typ IN ('channel_switched', 'name_changed')) NOT NULL,
	content BLOB,
	-- In UTC
	time DATETIME NOT NULL,
	-- Offset from UTC in seconds to the east
	timezone INTEGER NOT NULL,

	FOREIGN KEY(server, channel1) REFERENCES channels(server, id),
	FOREIGN KEY(server, channel2) REFERENCES channels(server, id)
);
CREATE INDEX events_server_index ON events(server);
CREATE INDEX events_invoker_index ON events(invoker);
CREATE INDEX events_client_index ON events(invoker);

-- Connecting different tables

CREATE TABLE servers_clients (
	server BLOB NOT NULL REFERENCES servers(public_key),
	client BLOB NOT NULL REFERENCES clients(uid),
	icon INTEGER,
	avatar TEXT,
	-- In UTC
	last_seen DATETIME NOT NULL,
	-- Offset from UTC in seconds to the east
	timezone INTEGER NOT NULL,

	PRIMARY KEY(server, client)
);

CREATE TABLE server_chats (
	server BLOB NOT NULL PRIMARY KEY REFERENCES servers(public_key),
	chat INTEGER NOT NULL REFERENCES chats(id)
);
CREATE INDEX server_chats_chat_index ON server_chats(chat);

CREATE TABLE channel_chats (
	server BLOB NOT NULL,
	channel INTEGER NOT NULL,
	chat INTEGER NOT NULL REFERENCES chats(id),

	PRIMARY KEY(server, channel),
	FOREIGN KEY(server, channel) REFERENCES channels(server, id)
);
CREATE INDEX channel_chats_chat_index ON channel_chats(chat);

CREATE TABLE client_chats (
	server BLOB NOT NULL REFERENCES servers(public_key),
	-- Message author
	client BLOB NOT NULL REFERENCES clients(uid),
	chat INTEGER NOT NULL REFERENCES chats(id),

	PRIMARY KEY(server, client)
);
CREATE INDEX client_chats_chat_index ON client_chats(chat);

CREATE TABLE client_pokes (
	server BLOB NOT NULL REFERENCES servers(public_key),
	-- Message author
	client BLOB NOT NULL REFERENCES clients(uid),
	chat INTEGER NOT NULL REFERENCES chats(id),

	PRIMARY KEY(server, client)
);
CREATE INDEX client_pokes_chat_index ON client_pokes(chat);
