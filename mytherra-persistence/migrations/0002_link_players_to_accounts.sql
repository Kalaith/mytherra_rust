-- Account linking (GDD 7.3): a deity may bind to a WebHatchery account so the
-- same god resumes across devices. A NULL `account_id` is a pure guest; a set
-- one is the account that owns this deity. The UNIQUE key holds one deity per
-- account (MySQL permits many NULLs, so every guest still coexists), and is the
-- index the resume path looks a returning account up by.
ALTER TABLE players
    ADD COLUMN account_id VARCHAR(128) NULL AFTER player_id,
    ADD UNIQUE KEY uq_players_account_id (account_id);
