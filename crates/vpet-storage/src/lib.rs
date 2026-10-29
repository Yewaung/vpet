//! Local SQLite persistence. All pet data lives in one file under the
//! user's app-data directory; deleting that file (and its backups) deletes
//! the pet. See `backup.rs` for how a damaged file is recovered.

mod backup;

use std::fmt;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

pub use backup::Recovery;
use vpet_core::learning::InterestStat;
use vpet_core::memory::{DayLog, SHORT_TERM_RETENTION_DAYS};
use vpet_core::vocabulary::WordStat;
use vpet_core::{
    behavior::BehaviorState, ActivitySession, AppCategory, ChatLine, EventRecord, Genome, Interests,
    LifeStage, Memory, MemoryKind, Needs, NeedKind, Personality, PetEvent, PetState, Relationship,
    Trait, VisualDna, Vocabulary, WordSource,
};

/// Migrations run in order; the index + 1 is the schema version.
const MIGRATIONS: &[&str] = &[
    include_str!("schema_v1.sql"),
    include_str!("schema_v2.sql"),
    include_str!("schema_v3.sql"),
    include_str!("schema_v4.sql"),
    include_str!("schema_v5.sql"),
    include_str!("schema_v6.sql"),
    include_str!("schema_v7.sql"),
];
/// Raw events are a rolling window, not an archive.
const MAX_EVENTS: i64 = 5000;
/// Chat lines kept for context; older lines are dropped.
const MAX_CONVERSATION_LINES: i64 = 200;
/// Raw activity sessions are kept this long; learned interests persist.
const ACTIVITY_RETENTION_DAYS: i64 = 30;
const DAY_MS: i64 = 86_400_000;

#[derive(Debug)]
pub enum Error {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    Corrupt(String),
    /// Saved by a newer VPet (after going back to an older version). Never
    /// damage: the save is left exactly as it is.
    TooNew,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Sql(e) => write!(f, "database error: {e}"),
            Error::Json(e) => write!(f, "stored data is malformed: {e}"),
            Error::Io(e) => write!(f, "file error: {e}"),
            Error::Corrupt(m) => write!(f, "stored data is invalid: {m}"),
            Error::TooNew => write!(f, "Your pet was saved by a newer version of VPet, so this older version can't open it. Install the latest version from https://github.com/Yewaung/vpet/releases. Your pet is safe and hasn't been changed."),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Sql(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StoredMemory {
    pub id: i64,
    #[serde(flatten)]
    pub memory: Memory,
}

/// One retired pet in the family tree: (generation, name, final stage, egg kind).
pub type Ancestor = (u32, Option<String>, String, Option<String>);

/// One life-stage transition (spec §43).
#[derive(Clone, Debug, PartialEq)]
pub struct EvolutionRecord {
    pub at_ms: i64,
    /// The pet's genome generation after the change.
    pub generation: u32,
    pub from: LifeStage,
    pub to: LifeStage,
    pub changes: Vec<(String, f64)>,
    /// The body it grew into (older records may not have it).
    pub genome: Option<Genome>,
    pub form_name: Option<String>,
    pub description: Option<String>,
    pub reasons: Vec<String>,
}

/// Care, lineage, mind and identity, stored together as `pet.life_json`.
#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
struct Life {
    care: vpet_core::care::Care,
    lineage: vpet_core::evolution::Lineage,
    mind: vpet_core::mind::Mind,
    gender: Option<vpet_core::Gender>,
    temper_lean: f64,
    last_together_ms: i64,
    last_ignored_ms: i64,
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Opens the database. With WAL and `synchronous = FULL` every committed
    /// transaction survives a crash or power cut; a torn write can only lose
    /// the transaction in flight, never corrupt what came before.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut s = Self { conn };
        s.migrate()?;
        Ok(s)
    }

    fn migrate(&mut self) -> Result<()> {
        let version: i64 = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > MIGRATIONS.len() as i64 {
            return Err(Error::TooNew);
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(())
    }

    /// Deletes the database, its WAL side files, its backups and any damaged
    /// copies set aside by recovery. The connection must be dropped first.
    /// Missing files are fine: the goal is that nothing remains.
    pub fn destroy(path: &Path) -> Result<()> {
        let mut paths = vec![path.to_path_buf()];
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut p = path.as_os_str().to_owned();
            p.push(suffix);
            paths.push(p.into());
        }
        paths.extend(backup::side_files(path));
        for p in paths {
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    pub fn load_pet(&self) -> Result<Option<PetState>> {
        let row = self
            .conn
            .query_row(
                "SELECT seed, rng_state, stage, created_at, hatched_at, age_secs, incubation,
                        generation, care_count, last_seen, needs_json, behavior_json,
                        day_log_json, critical_json, conversations, language_stage, name, goals_json,
                        user_name, curiosity_json, stage_age_secs, life_json
                 FROM pet WHERE id = 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                        r.get::<_, f64>(5)?,
                        r.get::<_, f64>(6)?,
                        r.get::<_, u32>(7)?,
                        r.get::<_, u32>(8)?,
                        r.get::<_, i64>(9)?,
                        r.get::<_, String>(10)?,
                        r.get::<_, String>(11)?,
                        r.get::<_, String>(12)?,
                        r.get::<_, String>(13)?,
                        r.get::<_, u32>(14)?,
                        r.get::<_, u8>(15)?,
                        r.get::<_, Option<String>>(16)?,
                        r.get::<_, String>(17)?,
                        (r.get::<_, Option<String>>(18)?, r.get::<_, String>(19)?, r.get::<_, f64>(20)?, r.get::<_, String>(21)?),
                    ))
                },
            )
            .optional()?;
        let Some((seed, rng_state, stage, created, hatched, age, incubation, generation, care, last_seen, needs, behavior, day_log, critical, conversations, language_stage, name, goals, (user_name, curiosity, stage_age_secs, life))) = row else {
            return Ok(None);
        };

        let stage = LifeStage::from_key(&stage).ok_or_else(|| Error::Corrupt(format!("unknown stage {stage}")))?;
        let life: Life = serde_json::from_str(&life)?;
        let (genome, visual_dna) = self.load_genome(generation)?;

        let mut personality = Personality::neutral();
        let mut stmt = self.conn.prepare("SELECT trait, value FROM personality")?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))? {
            let (key, value) = row?;
            if let Some(t) = Trait::from_key(&key) {
                personality.set(t, value);
            }
        }

        let relationship = self
            .conn
            .query_row("SELECT trust, affection, familiarity FROM relationship WHERE id = 1", [], |r| {
                Ok(Relationship {
                    trust: r.get(0)?,
                    affection: r.get(1)?,
                    familiarity: r.get(2)?,
                })
            })
            .optional()?
            .unwrap_or_default();

        Ok(Some(PetState {
            seed: seed as u64,
            // Rng::new(x) resumes from raw state x.
            rng: vpet_core::rng::Rng::new(rng_state as u64),
            stage,
            created_at_ms: created,
            hatched_at_ms: hatched,
            age_secs: age,
            incubation,
            generation,
            needs: serde_json::from_str::<Needs>(&needs)?,
            personality,
            relationship,
            genome,
            visual_dna,
            behavior: serde_json::from_str::<BehaviorState>(&behavior)?,
            care_count: care,
            day_log: serde_json::from_str::<DayLog>(&day_log)?,
            critical: serde_json::from_str::<Vec<NeedKind>>(&critical)?,
            last_seen_ms: last_seen,
            vocabulary: self.load_vocabulary()?,
            interests: self.load_interests()?,
            conversations,
            language_stage,
            name,
            goals: serde_json::from_str(&goals)?,
            watching: None,
            watch_secs: 0.0,
            quiet: false,
            hushed: false,
            user_name,
            curiosity: serde_json::from_str(&curiosity)?,
            stage_age_secs,
            care: life.care,
            lineage: life.lineage,
            mind: life.mind,
            gender: life.gender,
            temper_lean: life.temper_lean,
            last_together_ms: life.last_together_ms,
            last_ignored_ms: life.last_ignored_ms,
        }))
    }

    fn load_vocabulary(&self) -> Result<Vocabulary> {
        let mut v = Vocabulary::default();
        let mut stmt = self.conn.prepare("SELECT word, exposures, learned_at, source FROM vocabulary")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, Option<i64>>(2)?, r.get::<_, String>(3)?))
        })?;
        for row in rows {
            let (word, exposures, learned_at_ms, source) = row?;
            let source = WordSource::from_key(&source).ok_or_else(|| Error::Corrupt(format!("unknown word source {source}")))?;
            v.words.insert(word, WordStat { exposures, learned_at_ms, source });
        }
        Ok(v)
    }

    fn load_interests(&self) -> Result<Interests> {
        let mut i = Interests::default();
        let mut stmt = self.conn.prepare("SELECT category, stat_json FROM interest")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (cat, json) = row?;
            let cat = AppCategory::from_key(&cat).ok_or_else(|| Error::Corrupt(format!("unknown category {cat}")))?;
            i.stats.insert(cat, serde_json::from_str::<InterestStat>(&json)?);
        }
        Ok(i)
    }

    fn load_genome(&self, generation: u32) -> Result<(Genome, VisualDna)> {
        let (g, v): (String, String) = self
            .conn
            .query_row(
                "SELECT genome_json, visual_dna_json FROM creature_genome WHERE generation = ?1",
                [generation],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| Error::Corrupt(format!("missing genome for generation {generation}")))?;
        Ok((serde_json::from_str(&g)?, serde_json::from_str(&v)?))
    }

    /// Saves the whole pet atomically. Refuses to persist an invalid genome.
    pub fn save_pet(&mut self, s: &PetState) -> Result<()> {
        s.genome
            .validate()
            .map_err(|v| Error::Corrupt(format!("refusing to save invalid genome: {v:?}")))?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO pet (id, seed, rng_state, stage, created_at, hatched_at, age_secs, incubation,
                              generation, care_count, last_seen, needs_json, behavior_json,
                              day_log_json, critical_json, conversations, language_stage, name, goals_json,
                              user_name, curiosity_json, stage_age_secs, life_json)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
             ON CONFLICT(id) DO UPDATE SET
                seed = excluded.seed, rng_state = excluded.rng_state, stage = excluded.stage,
                created_at = excluded.created_at, hatched_at = excluded.hatched_at,
                age_secs = excluded.age_secs, incubation = excluded.incubation,
                generation = excluded.generation, care_count = excluded.care_count,
                last_seen = excluded.last_seen, needs_json = excluded.needs_json,
                behavior_json = excluded.behavior_json, day_log_json = excluded.day_log_json,
                critical_json = excluded.critical_json,
                conversations = excluded.conversations, language_stage = excluded.language_stage,
                name = excluded.name, goals_json = excluded.goals_json,
                user_name = excluded.user_name, curiosity_json = excluded.curiosity_json,
                stage_age_secs = excluded.stage_age_secs, life_json = excluded.life_json",
            params![
                s.seed as i64,
                s.rng.state() as i64,
                s.stage.key(),
                s.created_at_ms,
                s.hatched_at_ms,
                s.age_secs,
                s.incubation,
                s.generation,
                s.care_count,
                s.last_seen_ms,
                serde_json::to_string(&s.needs)?,
                serde_json::to_string(&s.behavior)?,
                serde_json::to_string(&s.day_log)?,
                serde_json::to_string(&s.critical)?,
                s.conversations,
                s.language_stage,
                s.name,
                serde_json::to_string(&s.goals)?,
                s.user_name,
                serde_json::to_string(&s.curiosity)?,
                s.stage_age_secs,
                serde_json::to_string(&Life {
                    care: s.care.clone(),
                    lineage: s.lineage.clone(),
                    mind: s.mind.clone(),
                    gender: s.gender,
                    temper_lean: s.temper_lean,
                    last_together_ms: s.last_together_ms,
                    last_ignored_ms: s.last_ignored_ms,
                })?,
            ],
        )?;
        for (word, w) in &s.vocabulary.words {
            tx.execute(
                "INSERT INTO vocabulary (word, exposures, learned_at, source) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(word) DO UPDATE SET exposures = excluded.exposures,
                    learned_at = excluded.learned_at",
                params![word, w.exposures, w.learned_at_ms, w.source.key()],
            )?;
        }
        for (cat, stat) in &s.interests.stats {
            tx.execute(
                "INSERT INTO interest (category, stat_json) VALUES (?1, ?2)
                 ON CONFLICT(category) DO UPDATE SET stat_json = excluded.stat_json",
                params![cat.key(), serde_json::to_string(stat)?],
            )?;
        }
        for t in Trait::ALL {
            tx.execute(
                "INSERT INTO personality (trait, value) VALUES (?1, ?2)
                 ON CONFLICT(trait) DO UPDATE SET value = excluded.value",
                params![t.key(), s.personality.get(t)],
            )?;
        }
        let r = &s.relationship;
        tx.execute(
            "INSERT INTO relationship (id, trust, affection, familiarity) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET trust = excluded.trust,
                affection = excluded.affection, familiarity = excluded.familiarity",
            params![r.trust, r.affection, r.familiarity],
        )?;
        tx.execute(
            "INSERT INTO creature_genome (generation, genome_json, visual_dna_json, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(generation) DO UPDATE SET genome_json = excluded.genome_json,
                visual_dna_json = excluded.visual_dna_json",
            params![
                s.generation,
                serde_json::to_string(&s.genome)?,
                serde_json::to_string(&s.visual_dna)?,
                s.last_seen_ms,
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Persists a tick's events and memories. Stage changes also enter the
    /// evolution history, with a snapshot of the body `pet` grew into.
    pub fn record(&mut self, events: &[EventRecord], memories: &[Memory], pet: &PetState) -> Result<()> {
        if events.is_empty() && memories.is_empty() {
            return Ok(());
        }
        let genome = serde_json::to_string(&pet.genome)?;
        let tx = self.conn.transaction()?;
        for e in events {
            tx.execute("INSERT INTO events (at, name, payload_json) VALUES (?1, ?2, ?3)", params![e.at_ms, e.event.name(), serde_json::to_string(&e.event)?])?;
            let none: &[String] = &[];
            let stage_change = match &e.event {
                PetEvent::StageChanged { from, to } => Some((from, to, "[]".to_string(), None, None, none)),
                PetEvent::Evolved { from, to, changes, form_name, description, reasons } => {
                    Some((from, to, serde_json::to_string(changes)?, form_name.as_deref(), Some(description.as_str()), reasons.as_slice()))
                }
                _ => None,
            };
            if let Some((from, to, changes, form_name, description, reasons)) = stage_change {
                tx.execute(
                    "INSERT INTO evolution (generation, from_stage, to_stage, genome_changes_json, at,
                                            genome_json, form_name, description, reasons_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![pet.generation, from.key(), to.key(), changes, e.at_ms, genome, form_name, description, serde_json::to_string(reasons)?],
                )?;
            }
        }
        for m in memories {
            tx.execute(
                "INSERT INTO memory (kind, tag, summary, importance, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![m.kind.key(), m.tag, m.summary, m.importance, m.created_at_ms],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn record_activity(&self, s: &ActivitySession) -> Result<()> {
        self.conn.execute(
            "INSERT INTO activity (started_at, duration_secs, app, category, confidence) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![s.started_ms, s.duration_secs, s.app, s.category.key(), s.confidence],
        )?;
        Ok(())
    }

    /// Minutes per category since `since_ms`, largest first.
    pub fn activity_summary(&self, since_ms: i64) -> Result<Vec<(String, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT category, SUM(duration_secs) / 60.0 AS mins FROM activity
             WHERE started_at >= ?1 GROUP BY category ORDER BY mins DESC",
        )?;
        let rows = stmt.query_map([since_ms], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn append_conversation(&self, at_ms: i64, line: &ChatLine) -> Result<()> {
        self.conn.execute(
            "INSERT INTO conversation (at, from_user, text) VALUES (?1, ?2, ?3)",
            params![at_ms, line.from_user, line.text],
        )?;
        self.conn.execute(
            "DELETE FROM conversation WHERE id <= (SELECT MAX(id) FROM conversation) - ?1",
            [MAX_CONVERSATION_LINES],
        )?;
        Ok(())
    }

    /// Most recent chat lines, oldest first.
    pub fn recent_conversation(&self, limit: u32) -> Result<Vec<ChatLine>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_user, text FROM (SELECT id, from_user, text FROM conversation ORDER BY id DESC LIMIT ?1)
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([limit], |r| Ok(ChatLine { from_user: r.get(0)?, text: r.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Starts the next generation: the parent joins the family tree, and
    /// everything that belonged to it personally (memories, words, chat,
    /// genome history) makes room for the child. Settings, and what was
    /// learned about the user's habits, stay with the family.
    pub fn begin_next_generation(&mut self, parent: &PetState, child: &PetState, now_ms: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        let personality: Vec<(&str, f64)> = Trait::ALL.iter().map(|t| (t.key(), parent.personality.get(*t))).collect();
        tx.execute(
            "INSERT INTO ancestor (generation, name, stage, genome_json, personality_json, born_at, retired_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                parent.lineage.generation,
                parent.name,
                parent.stage.key(),
                serde_json::to_string(&parent.genome)?,
                serde_json::to_string(&personality)?,
                parent.hatched_at_ms.unwrap_or(parent.created_at_ms),
                now_ms,
            ],
        )?;
        for table in ["pet", "personality", "relationship", "creature_genome", "evolution", "memory", "events", "vocabulary", "conversation"] {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
        tx.commit()?;
        self.save_pet(child)
    }

    /// The family tree, oldest first.
    pub fn ancestors(&self) -> Result<Vec<Ancestor>> {
        let mut stmt = self.conn.prepare("SELECT generation, name, stage, genome_json FROM ancestor ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (gen, name, stage, genome) = row?;
            let egg = serde_json::from_str::<Genome>(&genome).ok().and_then(|g| g.egg).map(|e| e.key().to_string());
            out.push((gen, name, stage, egg));
        }
        Ok(out)
    }

    /// Id of the newest chat line (0 if none): lets the chat window spot new lines.
    pub fn last_conversation_id(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COALESCE(MAX(id), 0) FROM conversation", [], |r| r.get(0))?)
    }

    /// Candidate memories for retrieval: every long-term memory plus recent ones.
    pub fn retrieval_pool(&self, limit: u32) -> Result<Vec<Memory>> {
        Ok(self.recent_memories(limit)?.into_iter().map(|m| m.memory).collect())
    }

    /// Stage changes, oldest first. Rows with an unknown stage are skipped.
    pub fn evolution_history(&self) -> Result<Vec<EvolutionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT at, generation, from_stage, to_stage, genome_changes_json, genome_json, form_name, description, reasons_json
             FROM evolution ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                (r.get::<_, i64>(0)?, r.get::<_, u32>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?),
                (r.get::<_, String>(4)?, r.get::<_, Option<String>>(5)?, r.get::<_, Option<String>>(6)?),
                (r.get::<_, Option<String>>(7)?, r.get::<_, String>(8)?),
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let ((at_ms, generation, from, to), (changes, genome, form_name), (description, reasons)) = row?;
            let (Some(from), Some(to)) = (LifeStage::from_key(&from), LifeStage::from_key(&to)) else {
                continue;
            };
            let changes: Vec<vpet_core::evolution::GeneChange> = serde_json::from_str(&changes).unwrap_or_default();
            out.push(EvolutionRecord {
                at_ms,
                generation,
                from,
                to,
                changes: changes.into_iter().map(|c| (c.gene, c.delta)).collect(),
                genome: genome.and_then(|g| serde_json::from_str(&g).ok()),
                form_name,
                description,
                reasons: serde_json::from_str(&reasons).unwrap_or_default(),
            });
        }
        Ok(out)
    }

    /// The genome the pet had in `generation`, if it was saved.
    pub fn genome_at(&self, generation: u32) -> Result<Option<Genome>> {
        let json: Option<String> =
            self.conn.query_row("SELECT genome_json FROM creature_genome WHERE generation = ?1", [generation], |r| r.get(0)).optional()?;
        Ok(json.and_then(|g| serde_json::from_str(&g).ok()))
    }

    /// Most recent events, newest first. Unreadable rows are skipped.
    pub fn recent_events(&self, limit: u32) -> Result<Vec<EventRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT at, payload_json FROM events ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (at_ms, json) = row?;
            if let Ok(event) = serde_json::from_str::<PetEvent>(&json) {
                out.push(EventRecord { at_ms, event });
            }
        }
        Ok(out)
    }

    pub fn memory_tags(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT tag FROM memory")?;
        let tags = stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        Ok(tags)
    }

    pub fn recent_memories(&self, limit: u32) -> Result<Vec<StoredMemory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, tag, summary, importance, created_at FROM memory
             ORDER BY created_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, u8>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind, tag, summary, importance, created_at_ms) = row?;
            let kind = MemoryKind::from_key(&kind).ok_or_else(|| Error::Corrupt(format!("unknown memory kind {kind}")))?;
            out.push(StoredMemory {
                id,
                memory: Memory {
                    kind,
                    tag,
                    summary,
                    importance,
                    created_at_ms,
                },
            });
        }
        Ok(out)
    }

    /// Forgets expired short-term memories and trims the raw event window.
    /// Returns how many rows were removed.
    pub fn prune(&self, now_ms: i64) -> Result<usize> {
        let cutoff = now_ms - SHORT_TERM_RETENTION_DAYS * DAY_MS;
        let forgotten = self.conn.execute(
            "DELETE FROM memory WHERE importance < ?1 AND created_at < ?2",
            params![vpet_core::memory::LONG_TERM_IMPORTANCE, cutoff],
        )?;
        let trimmed = self.conn.execute(
            "DELETE FROM events WHERE id <= (SELECT MAX(id) FROM events) - ?1",
            [MAX_EVENTS],
        )?;
        let old_activity = self.conn.execute(
            "DELETE FROM activity WHERE started_at < ?1",
            [now_ms - ACTIVITY_RETENTION_DAYS * DAY_MS],
        )?;
        Ok(forgotten + trimmed + old_activity)
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn count(&self, table: &str) -> Result<i64> {
        const TABLES: &[&str] = &[
            "pet", "personality", "relationship", "creature_genome", "evolution", "memory", "events",
            "settings", "vocabulary", "interest", "activity", "conversation", "ancestor",
        ];
        if !TABLES.contains(&table) {
            return Err(Error::Corrupt(format!("unknown table {table}")));
        }
        Ok(self.conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vpet_core::{Interaction, Pet, TickInput};

    fn input(now_ms: i64) -> TickInput {
        TickInput { now_ms, dt_secs: 1.0, time_scale: 1.0, local_hour: 12, local_day: 1, near_cursor: false, watching: None, quiet: false, user_idle_secs: 0.0, hushed: false }
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut s = Storage::open_in_memory().unwrap();
        s.migrate().unwrap();
        assert_eq!(s.count("pet").unwrap(), 0);
    }

    #[test]
    fn empty_database_has_no_pet() {
        assert!(Storage::open_in_memory().unwrap().load_pet().unwrap().is_none());
    }

    #[test]
    fn pet_round_trips_exactly() {
        let mut s = Storage::open_in_memory().unwrap();
        let mut pet = Pet::new(PetState::new_egg_of(vpet_core::EggKind::Earth, 0, u64::MAX - 3), Vec::new());
        pet.state.incubation = 1.0;
        for i in 1..200 {
            pet.tick(&input(i * 1000));
            if i % 10 == 0 {
                pet.interact(Interaction::Poke, i * 1000);
            }
        }
        s.save_pet(&pet.state).unwrap();
        let loaded = s.load_pet().unwrap().unwrap();
        assert_eq!(loaded, pet.state);
    }

    #[test]
    fn learning_state_round_trips() {
        let mut s = Storage::open_in_memory().unwrap();
        let mut pet = Pet::new(PetState::new_egg_of(vpet_core::EggKind::Earth, 0, 5), Vec::new());
        pet.state.incubation = 1.0;
        pet.tick(&input(1000));
        pet.hear_user("hello food hello food hello food, I call you Kiba", 2000);
        let session = ActivitySession {
            app: "code".into(),
            category: AppCategory::Programming,
            confidence: 0.9,
            started_ms: 0,
            duration_secs: 1800.0,
            local_hour: 20,
            local_day: 1,
        };
        pet.observe_activity(&session, 3000);
        pet.observe_activity(&session, 4000);
        s.record_activity(&session).unwrap();
        s.save_pet(&pet.state).unwrap();
        let loaded = s.load_pet().unwrap().unwrap();
        assert_eq!(loaded, pet.state);
        assert!(loaded.vocabulary.knows("food"));
        assert_eq!(s.activity_summary(0).unwrap()[0].0, "programming");
    }

    #[test]
    fn conversation_is_a_bounded_window() {
        let s = Storage::open_in_memory().unwrap();
        for i in 0..(MAX_CONVERSATION_LINES + 20) {
            s.append_conversation(i, &ChatLine { from_user: i % 2 == 0, text: format!("line {i}") }).unwrap();
        }
        assert_eq!(s.count("conversation").unwrap(), MAX_CONVERSATION_LINES);
        let recent = s.recent_conversation(2).unwrap();
        assert_eq!(recent[1].text, format!("line {}", MAX_CONVERSATION_LINES + 19));
    }

    #[test]
    fn recent_events_round_trip_newest_first() {
        let mut s = Storage::open_in_memory().unwrap();
        let evs = vec![EventRecord { at_ms: 1, event: PetEvent::Petted }, EventRecord { at_ms: 2, event: PetEvent::WordLearned { word: "hi".into() } }];
        s.record(&evs, &[], &PetState::new_egg(0, 1)).unwrap();
        let back = s.recent_events(10).unwrap();
        assert_eq!(back[0], evs[1]);
        assert_eq!(back[1], evs[0]);
    }

    #[test]
    fn invalid_genome_is_never_saved() {
        let mut s = Storage::open_in_memory().unwrap();
        let mut state = PetState::new_egg(0, 1);
        state.genome.features.tail = 3.0;
        assert!(s.save_pet(&state).is_err());
        assert_eq!(s.count("pet").unwrap(), 0);
    }

    #[test]
    fn events_memories_and_evolution_are_recorded() {
        let mut s = Storage::open_in_memory().unwrap();
        let mut pet = Pet::new(PetState::new_egg_of(vpet_core::EggKind::Earth, 0, 1), Vec::new());
        pet.state.incubation = 1.0;
        let out = pet.tick(&input(1000));
        s.record(&out.events, &out.memories, &pet.state).unwrap();
        assert_eq!(s.count("evolution").unwrap(), 1);
        let hatch = &s.evolution_history().unwrap()[0];
        assert_eq!((hatch.from, hatch.to), (LifeStage::Egg, LifeStage::Baby));
        assert_eq!(hatch.genome.as_ref(), Some(&pet.state.genome), "it keeps the body it grew into, for portraits");
        assert!(s.memory_tags().unwrap().contains(&"first:hatch".to_string()));
        assert_eq!(s.recent_memories(10).unwrap()[0].memory.importance, 5);
    }

    #[test]
    fn evolutions_keep_their_form_story_and_reasons() {
        let mut s = Storage::open_in_memory().unwrap();
        let mut pet = PetState::new_egg_of(vpet_core::EggKind::Wind, 0, 3);
        pet.generation = 2;
        let evolved = EventRecord {
            at_ms: 9,
            event: PetEvent::Evolved {
                from: LifeStage::Child,
                to: LifeStage::Rookie,
                description: "I chased the wind.".into(),
                changes: vec![vpet_core::evolution::GeneChange { gene: "features.horns".into(), delta: 0.1 }],
                form_name: Some("Breezle".into()),
                reasons: vec!["Horns grew because I've grown brave.".into()],
            },
        };
        s.record(&[evolved], &[], &pet).unwrap();
        s.save_pet(&pet).unwrap();
        let r = &s.evolution_history().unwrap()[0];
        assert_eq!(r.generation, 2);
        assert_eq!(r.form_name.as_deref(), Some("Breezle"));
        assert_eq!(r.description.as_deref(), Some("I chased the wind."));
        assert_eq!(r.reasons, vec!["Horns grew because I've grown brave.".to_string()]);
        assert_eq!(r.changes, vec![("features.horns".to_string(), 0.1)]);
        assert_eq!(s.genome_at(2).unwrap(), Some(pet.genome.clone()));
        assert_eq!(s.genome_at(9).unwrap(), None);
    }

    #[test]
    fn prune_forgets_old_trivia_but_keeps_important_memories() {
        let mut s = Storage::open_in_memory().unwrap();
        let mk = |importance, tag: &str| Memory { kind: MemoryKind::Episodic, tag: tag.into(), summary: "x".into(), importance, created_at_ms: 0 };
        s.record(&[], &[mk(2, "trivia"), mk(5, "major")], &PetState::new_egg(0, 1)).unwrap();
        s.prune(30 * DAY_MS).unwrap();
        assert_eq!(s.memory_tags().unwrap(), vec!["major".to_string()]);
    }

    #[test]
    fn destroy_removes_every_file() {
        let dir = std::env::temp_dir().join(format!("vpet-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vpet.db");
        {
            let mut s = Storage::open(&path).unwrap();
            s.save_pet(&PetState::new_egg(0, 1)).unwrap();
        }
        Storage::destroy(&path).unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
        std::fs::remove_dir(&dir).unwrap();
    }
}
