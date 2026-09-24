-- Curiosity and evolution (spec 7, 41-43): the user's name, the pet's open
-- questions, and when its current life stage began.
ALTER TABLE pet ADD COLUMN user_name TEXT;
ALTER TABLE pet ADD COLUMN curiosity_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE pet ADD COLUMN stage_age_secs REAL NOT NULL DEFAULT 0;
