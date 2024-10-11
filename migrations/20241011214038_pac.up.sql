CREATE TABLE pac (
		hash TEXT NOT NULL,
		file TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_pac_hash ON pac(hash);
