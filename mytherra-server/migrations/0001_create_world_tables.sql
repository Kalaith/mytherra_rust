-- The authority server's persistence (GDD 6/8): the DB *is* the save.
--
-- The Rust core keeps the world as one monolithic, serde-serializable
-- `WorldState` (shared) with one `PlayerState` per deity, so v1 stores each as a
-- JSON document rather than the fully-relational schema GDD 6 targets for M3.
-- A server row therefore resumes the exact same deterministic sequence (GDD
-- 7.2): the world's `SeededRng` rides along inside the JSON.

-- The one shared world (GDD 6 "shared/global tables"): a singleton row.
-- `next_guest` is the monotonic guest-id counter, persisted so ids never collide
-- across a restart.
CREATE TABLE IF NOT EXISTS world_state (
    id         TINYINT UNSIGNED NOT NULL PRIMARY KEY,
    version    VARCHAR(64)      NOT NULL,
    next_guest BIGINT UNSIGNED  NOT NULL,
    state      JSON             NOT NULL,
    updated_at TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Per-deity private state (GDD 6 "per-player tables"): favor, champions, wagers,
-- Standing. `seq` preserves creation order so the tick processes deities in the
-- same order after a reload (determinism, GDD 5.8).
CREATE TABLE IF NOT EXISTS players (
    seq        BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    player_id  VARCHAR(64)     NOT NULL,
    state      JSON            NOT NULL,
    created_at TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (seq),
    UNIQUE KEY uq_players_player_id (player_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
