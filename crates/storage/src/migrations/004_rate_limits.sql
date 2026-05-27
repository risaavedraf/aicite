CREATE TABLE IF NOT EXISTS rate_limit_counters (
    key TEXT NOT NULL,
    route TEXT NOT NULL,
    window_start_epoch INTEGER NOT NULL,
    request_count INTEGER NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (key, route, window_start_epoch)
);
