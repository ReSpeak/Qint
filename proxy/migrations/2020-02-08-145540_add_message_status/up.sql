ALTER TABLE messages INSERT COLUMN
	status TEXT CHECK(status IN ('sending', 'success', 'error')) NOT NULL DEFAULT 'success',
