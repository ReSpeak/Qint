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

CREATE TABLE servers (
	public_key BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	-- Last used address
	address TEXT NOT NULL,
	icon INTEGER
);

CREATE TABLE channels (
	server BLOB NOT NULL REFERENCES servers(id),
	id INTEGER NOT NULL,
	parent INTEGER,
	name TEXT NOT NULL,
	icon INTEGER,
	deleted BOOLEAN NOT NULL DEFAULT false,

	PRIMARY KEY(server, id),
	FOREIGN KEY(server, parent) REFERENCES channels(server, id)
);

CREATE TABLE bookmarks (
	id INTEGER NOT NULL PRIMARY KEY,
	name TEXT,
	address TEXT NOT NULL,
	channel INTEGER,
	identity INTEGER NOT NULL REFERENCES identities(id),
	bookmark BOOLEAN NOT NULL DEFAULT false,
	last_used DATETIME,
	-- References the server if already connected once
	server BLOB,

	FOREIGN KEY(server, channel) REFERENCES channels(server, id)
);

CREATE TABLE messages (
	id INTEGER NOT NULL PRIMARY KEY,
	-- NULL if we got a message from the server
	invoker BLOB REFERENCES clients(uid),
	content TEXT NOT NULL,
	time DATETIME NOT NULL
);

CREATE TABLE events (
	id INTEGER NOT NULL PRIMARY KEY,
	server BLOB REFERENCES servers(id),
	invoker BLOB REFERENCES clients(uid),
	channel1 INTEGER NOT NULL,
	channel2 INTEGER NOT NULL,
	client BLOB REFERENCES clients(uid),
	typ TEXT CHECK(typ IN ('channel_switched', 'name_changed')) NOT NULL,
	content BLOB,
	time DATETIME NOT NULL,

	FOREIGN KEY(server, channel1) REFERENCES channels(server, id),
	FOREIGN KEY(server, channel2) REFERENCES channels(server, id)
);

-- Connecting different tables

CREATE TABLE servers_clients (
	server BLOB NOT NULL REFERENCES servers(id),
	client BLOB NOT NULL REFERENCES clients(uid),
	icon INTEGER,
	last_seen DATETIME NOT NULL,

	PRIMARY KEY(server, client)
);

CREATE TABLE server_messages (
	server BLOB NOT NULL REFERENCES servers(id),
	message INTEGER NOT NULL REFERENCES messages(id),

	PRIMARY KEY(server, message)
);

CREATE TABLE channel_messages (
	server BLOB NOT NULL,
	channel INTEGER NOT NULL,
	message INTEGER NOT NULL REFERENCES messages(id),

	PRIMARY KEY(server, channel, message),
	FOREIGN KEY(server, channel) REFERENCES channels(server, id)
);

CREATE TABLE client_messages (
	server BLOB NOT NULL REFERENCES servers(id),
	-- Message author
	client BLOB NOT NULL REFERENCES clients(uid),
	message INTEGER NOT NULL REFERENCES messages(id),

	PRIMARY KEY(server, client, message)
);
