ALTER TABLE clients RENAME TO _clients_old;

CREATE TABLE clients (
	-- sha1(base64(primary key))
	uid BLOB NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	public_key BLOB,
	custom_name TEXT
);

INSERT INTO clients (uid, name, public_key, custom_name)
	SELECT uid, name, public_key, custom_name
	FROM _clients_old;

DROP TABLE _clients_old;
