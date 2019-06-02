PRAGMA foreign_keys = ON;

CREATE TABLE identities (
	id INTEGER NOT NULL PRIMARY KEY,
	-- encrypted
	private_key BLOB NOT NULL,
	name TEXT NOT NULL DEFAULT "Default",
	offset INTEGER NOT NULL DEFAULT 0,
	client BLOB NOT NULL REFERENCES clients(uid)
);

CREATE TABLE clients (
	-- sha1(base64(primary key))
	uid BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	public_key TEXT,
	custom_name TEXT
);

CREATE TABLE channels (
	server INTEGER NOT NULL REFERENCES servers(id),
	id INTEGER NOT NULL,
	parent INTEGER,
	name TEXT NOT NULL,
	deleted BOOLEAN NOT NULL DEFAULT false,

	PRIMARY KEY(server, id),
	FOREIGN KEY(server, parent) REFERENCES channels(server, id)
);

CREATE TABLE servers (
	id INTEGER NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	-- Last used address
	address TEXT NOT NULL
);

CREATE TABLE bookmarks (
	id INTEGER NOT NULL PRIMARY KEY,
	name TEXT,
	address TEXT NOT NULL,
	channel INTEGER,
	identity INTEGER REFERENCES identities(id),
	bookmark BOOLEAN NOT NULL DEFAULT false,
	last_used DATETIME,
	-- References the server if already connected once
	server INTEGER,

	FOREIGN KEY(server, channel) REFERENCES channels(server, id)
);

CREATE TABLE messages (
	id INTEGER NOT NULL PRIMARY KEY,
	invoker BLOB NOT NULL REFERENCES clients(uid),
	content TEXT NOT NULL,
	time DATETIME NOT NULL
);

-- Connecting different tables

CREATE TABLE servers_clients (
	server INTEGER NOT NULL REFERENCES servers(id),
	client BLOB NOT NULL REFERENCES clients(uid),
	last_seen DATETIME NOT NULL,

	PRIMARY KEY(server, client)
);

CREATE TABLE server_messages (
	server INTEGER NOT NULL REFERENCES servers(id),
	message INTEGER NOT NULL REFERENCES messages(id),

	PRIMARY KEY(server, message)
);

CREATE TABLE channel_messages (
	server INTEGER NOT NULL,
	channel INTEGER NOT NULL,
	message INTEGER NOT NULL REFERENCES messages(id),

	PRIMARY KEY(server, channel, message),
	FOREIGN KEY(server, channel) REFERENCES channels(server, id)
);

CREATE TABLE client_messages (
	server INTEGER NOT NULL REFERENCES servers(id),
	-- Message author
	client BLOB NOT NULL REFERENCES clients(uid),
	message INTEGER NOT NULL REFERENCES messages(id),

	PRIMARY KEY(server, client, message)
);
