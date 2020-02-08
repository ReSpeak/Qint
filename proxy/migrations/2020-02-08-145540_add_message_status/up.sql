ALTER TABLE messages ADD
	status TEXT CHECK(status IN ('sending', 'success', 'error')) NOT NULL DEFAULT 'success';
