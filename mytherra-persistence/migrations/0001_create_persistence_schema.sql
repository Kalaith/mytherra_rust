-- The authority's persistence schema (GDD 6/8): the DB *is* the save.
--
-- Two dissociated domains. The shared world is decomposed by entity — every
-- collection is its own table, one row per entity — with the non-collection
-- state (year, *_seq counters, era, chronicle, RNG) in a single `world_core`
-- row. The per-deity player domain is relational: economy columns on `players`,
-- child tables for the roster and wagers, and identity in `player_registry`.
-- The two never share a row: a deity nudges the world; it is never in it.

-- ── Shared world ────────────────────────────────────────────────────────────

-- Non-collection world state as one document (small: scalars + era + chronicle
-- + the RNG that makes a reload resume the exact deterministic sequence).
CREATE TABLE world_core (
    id         TINYINT UNSIGNED NOT NULL PRIMARY KEY,
    data       JSON             NOT NULL,
    updated_at TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- One table per world collection. `ord` is the entity's index within its Vec,
-- so ORDER BY ord reconstructs the exact order the tick iterates (determinism).
CREATE TABLE world_regions              (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_settlements          (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_resource_nodes       (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_landmarks            (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_trade_routes         (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_buildings            (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_heroes               (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_houses               (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_orders               (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_saints               (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_festivals            (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_artifacts            (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_pending_consequences (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_era_history          (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_weather              (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_plagues              (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_monsters             (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_wars                 (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_vassalages           (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_prophecies           (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_pacts                (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_magic_paths          (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_myths                (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_myth_candidates      (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_civilization         (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_pantheon             (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE world_speculations         (ord INT UNSIGNED NOT NULL PRIMARY KEY, data JSON NOT NULL) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ── Per-deity player domain ─────────────────────────────────────────────────

-- Player identity: the monotonic guest-id counter. Deliberately its own row,
-- not a column on any world table.
CREATE TABLE player_registry (
    id         TINYINT UNSIGNED NOT NULL PRIMARY KEY,
    next_guest BIGINT UNSIGNED  NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- A deity's economy as real columns; `seq` preserves creation order so the tick
-- processes deities identically after a reload. Achievements ride as a document.
CREATE TABLE players (
    seq          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    player_id    VARCHAR(64)     NOT NULL,
    favor        BIGINT          NOT NULL,
    level        INT UNSIGNED    NOT NULL,
    experience   BIGINT          NOT NULL,
    favor_spent  BIGINT          NOT NULL,
    nudges       INT UNSIGNED    NOT NULL,
    achievements JSON            NOT NULL,
    created_at   TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP       NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (seq),
    UNIQUE KEY uq_players_player_id (player_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- A deity's cultivated roster (GDD 5.4), keyed to the deity, ordered within it.
CREATE TABLE player_champions (
    player_id VARCHAR(64)  NOT NULL,
    ord       INT UNSIGNED NOT NULL,
    data      JSON         NOT NULL,
    PRIMARY KEY (player_id, ord)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- A deity's placed wagers (GDD 5.5).
CREATE TABLE player_bets (
    player_id VARCHAR(64)  NOT NULL,
    ord       INT UNSIGNED NOT NULL,
    data      JSON         NOT NULL,
    PRIMARY KEY (player_id, ord)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
