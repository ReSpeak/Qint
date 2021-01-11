ALTER TABLE bookmarks RENAME TO _bookmarks_old;

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

INSERT INTO bookmarks (id, name, username, address, channel, identity, bookmark, last_used, timezone, server)
	SELECT id, name, username, address, channel, identity, bookmark, last_used, timezone, server
	FROM _bookmarks_old;

DROP TABLE _bookmarks_old;
CREATE INDEX bookmarks_identity_index ON bookmarks(identity);
CREATE INDEX bookmarks_server_index ON bookmarks(server);
CREATE INDEX bookmarks_channel_index ON bookmarks(server, channel);

-- Client changes
ALTER TABLE clients RENAME TO _clients_old;

CREATE TABLE clients (
	-- sha1(base64(primary key))
	uid BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	public_key BLOB,
	custom_name TEXT,
	volume REAL NOT NULL DEFAULT 1.0
);

INSERT INTO clients (uid, name, public_key, custom_name, volume)
	SELECT uid, name, public_key, custom_name, volume
	FROM _clients_old;

DROP TABLE _clients_old;
