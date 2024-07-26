CREATE TABLE IF NOT EXISTS users (
	username      TEXT NOT NULL PRIMARY KEY,
	password_hash TEXT NOT NULL,
	admin         BOOLEAN NOT NULL DEFAULT FALSE,
	active        BOOL NOT NULL DEFAULT TRUE,
	provisioned   BOOL NOT NULL DEFAULT FALSE,
	created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS domains (
	domain      TEXT NOT NULL PRIMARY KEY,
	catch_all   TEXT,
	public      BOOL NOT NULL DEFAULT FALSE,
	active      BOOL NOT NULL DEFAULT TRUE,
	owner       TEXT NOT NULL,
	provisioned BOOL NOT NULL DEFAULT FALSE,
	created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
	-- FOREIGN KEY (owner) REFERENCES users (username) ON DELETE CASCADE
	-- FOREIGN KEY (catch_all) REFERENCES mailboxes (address) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS aliases (
	address     TEXT NOT NULL PRIMARY KEY,
	-- associated domain. Technically redundant but required to do efficient JOIN with the domain table.
	domain      TEXT NOT NULL,
	target      TEXT NOT NULL,
	comment     TEXT NOT NULL,
	n_recv      INTEGER NOT NULL DEFAULT 0,
	n_sent      INTEGER NOT NULL DEFAULT 0,
	active      BOOLEAN NOT NULL DEFAULT TRUE,
	owner       TEXT NOT NULL,
	provisioned BOOL NOT NULL DEFAULT FALSE,
	created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
	-- FOREIGN KEY (target) REFERENCES mailboxes (address) ON DELETE CASCADE
	-- FOREIGN KEY (owner) REFERENCES users (username) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS mailboxes (
	address       TEXT NOT NULL PRIMARY KEY,
	-- associated domain. Technically redundant but required to do efficient JOIN with the domain table.
	domain        TEXT NOT NULL,
	password_hash TEXT NOT NULL,
	api_token     TEXT UNIQUE DEFAULT NULL,
	active        BOOL NOT NULL DEFAULT TRUE,
	owner         TEXT NOT NULL,
	provisioned   BOOL NOT NULL DEFAULT FALSE,
	created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
	-- FOREIGN KEY (owner) REFERENCES users (username) ON DELETE CASCADE
) WITHOUT ROWID;
