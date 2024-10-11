CREATE TABLE white_list (
	host TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_white_list_host ON white_list(host);
