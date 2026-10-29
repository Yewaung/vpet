-- Evolution portraits and explanations (spec 43-44): each transition keeps
-- the body it grew into, its form's name and story, and the real reasons.
ALTER TABLE evolution ADD COLUMN genome_json TEXT;
ALTER TABLE evolution ADD COLUMN form_name TEXT;
ALTER TABLE evolution ADD COLUMN description TEXT;
ALTER TABLE evolution ADD COLUMN reasons_json TEXT NOT NULL DEFAULT '[]';
