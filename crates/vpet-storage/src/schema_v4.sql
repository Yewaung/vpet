-- The pet's own goals (spec 19).
ALTER TABLE pet ADD COLUMN goals_json TEXT NOT NULL DEFAULT '{"current":null,"cooldown":1200.0,"completed":0}';
