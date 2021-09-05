-- Remove password from servers
ALTER TABLE servers RENAME TO _servers_old;

CREATE TABLE servers (
	public_key BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	-- Last used address
	address TEXT NOT NULL,
	icon INTEGER
);

INSERT INTO servers (public_key, name, address, icon)
	SELECT public_key, name, address, icon
	FROM _servers_old;

DROP TABLE _servers_old;

-- Remove password from channels
ALTER TABLE channels RENAME TO _channels_old;

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

INSERT INTO channels (server, id, parent, order_id, name, icon, deleted)
	SELECT server, id, parent, order_id, name, icon, deleted
	FROM _channels_old;

DROP TABLE _channels_old;
CREATE INDEX channels_server_index ON channels(server);
CREATE INDEX channels_parent_index ON channels(server, parent);
