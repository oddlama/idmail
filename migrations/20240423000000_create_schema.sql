CREATE TABLE IF NOT EXISTS users (
	username   TEXT NOT NULL PRIMARY KEY,
	password   TEXT NOT NULL,
	admin      BOOLEAN NOT NULL DEFAULT FALSE,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	active     BOOL NOT NULL DEFAULT TRUE
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS user_permissions (
	username TEXT NOT NULL,
	token    TEXT NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE domains (
	domain     TEXT NOT NULL PRIMARY KEY,
	owner      TEXT NOT NULL,
	catch_all  TEXT NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	active     BOOL NOT NULL DEFAULT TRUE
	-- FOREIGN KEY (owner) REFERENCES users (username) ON DELETE CASCADE
	-- FOREIGN KEY (catch_all) REFERENCES mailboxes (address) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS aliases (
	address    TEXT NOT NULL PRIMARY KEY,
	target     TEXT NOT NULL,
	comment    TEXT NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	active     BOOLEAN NOT NULL DEFAULT TRUE
	-- FOREIGN KEY (to) REFERENCES mailboxes (address) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE mailboxes (
	address    TEXT NOT NULL PRIMARY KEY,
	owner      TEXT NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	active     BOOL NOT NULL DEFAULT TRUE
	-- FOREIGN KEY (owner) REFERENCES users (username) ON DELETE CASCADE
) WITHOUT ROWID;

--user admin
--user malte
--domains schmitz.sh
--domains privacymail.sh
--mailbox malte@schmitz.sh owner=malte
--alias a.b@privacymail.sh malte@schmitz.sh
--alias ccaedfaer@privacymail.sh malte@schmitz.sh
