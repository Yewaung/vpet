//! Crash safety beyond WAL: rolling backups and automatic recovery.
//!
//! A consistent copy of the database is written with `VACUUM INTO` and the
//! last two are kept (`vpet.db.bak1` newest, `vpet.db.bak2`). When the
//! database won't open, fails SQLite's integrity check or holds a pet that
//! can't be read, the damaged files are set aside as `vpet.db.broken-<ms>`
//! and the newest good backup takes their place. A pet is never lost to one
//! bad write, and nothing is thrown away without a copy.
//!
//! Backups are pet data: `Storage::destroy` removes them too.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::{Error, Result, Storage};

/// Backups kept, newest first.
const KEEP: usize = 2;
const WAL_FILES: [&str; 2] = ["-wal", "-shm"];

/// What opening the database had to do.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Recovery {
    /// The database was fine.
    #[default]
    None,
    /// It was damaged and was restored from a backup `age` old. The damaged
    /// file was kept at `broken`.
    Restored { age: Duration, broken: PathBuf },
    /// It was damaged and no backup could be read: a fresh egg starts. The
    /// damaged file was kept at `broken`.
    StartedFresh { broken: PathBuf },
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(suffix);
    p.into()
}

fn backup_path(db: &Path, i: usize) -> PathBuf {
    with_suffix(db, &format!(".bak{i}"))
}

/// Every backup, temporary copy and set-aside damaged file for `db`.
pub(crate) fn side_files(db: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = (1..=KEEP).map(|i| backup_path(db, i)).collect();
    out.push(with_suffix(db, ".bak-tmp"));
    let (Some(dir), Some(name)) = (db.parent(), db.file_name().and_then(|n| n.to_str())) else {
        return out;
    };
    let broken = format!("{name}.broken-");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.file_name().to_str().is_some_and(|n| n.starts_with(&broken)) {
                out.push(e.path());
            }
        }
    }
    out
}

fn age_of(path: &Path) -> Option<Duration> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    SystemTime::now().duration_since(modified).ok().or(Some(Duration::ZERO))
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Moves the database and its WAL files out of the way, keeping them.
fn set_aside(db: &Path, now_ms: i64) -> Result<PathBuf> {
    let broken = with_suffix(db, &format!(".broken-{now_ms}"));
    std::fs::rename(db, &broken)?;
    for suffix in WAL_FILES {
        let side = with_suffix(db, suffix);
        if side.exists() {
            std::fs::rename(&side, with_suffix(&broken, suffix))?;
        }
    }
    Ok(broken)
}

impl Storage {
    /// Opens `db`, checking that it is healthy and its pet can be read. A
    /// damaged database is set aside and replaced by the newest good backup,
    /// or by a fresh one if no backup can be read.
    pub fn open_checked(db: &Path, now_ms: i64) -> Result<(Storage, Recovery)> {
        if !db.exists() {
            return Ok((Storage::open(db)?, Recovery::None));
        }
        let damage = match Storage::open(db).and_then(|s| s.check().map(|()| s)) {
            Ok(s) => return Ok((s, Recovery::None)),
            Err(Error::TooNew) => return Err(Error::TooNew),
            Err(e) => e,
        };
        eprintln!("vpet: the pet's database is damaged ({damage}); recovering");
        let broken = set_aside(db, now_ms)?;
        for i in 1..=KEEP {
            let backup = backup_path(db, i);
            let Some(age) = age_of(&backup) else { continue };
            std::fs::copy(&backup, db)?;
            match Storage::open(db).and_then(|s| s.check().map(|()| s)) {
                Ok(s) => return Ok((s, Recovery::Restored { age, broken })),
                Err(e) => {
                    eprintln!("vpet: backup {} is unusable too: {e}", backup.display());
                    for suffix in ["", WAL_FILES[0], WAL_FILES[1]] {
                        remove_if_present(&with_suffix(db, suffix))?;
                    }
                }
            }
        }
        Ok((Storage::open(db)?, Recovery::StartedFresh { broken }))
    }

    /// SQLite's own integrity check, plus reading the pet back.
    fn check(&self) -> Result<()> {
        let verdict: String = self.conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if verdict != "ok" {
            return Err(Error::Corrupt(format!("integrity check: {verdict}")));
        }
        self.load_pet().map(|_| ())
    }

    /// Writes a consistent copy of the database next to it and rotates the
    /// older copies. Safe while the pet is running.
    pub fn backup(&self, db: &Path) -> Result<()> {
        let tmp = with_suffix(db, ".bak-tmp");
        remove_if_present(&tmp)?;
        let target = tmp.to_str().ok_or_else(|| Error::Corrupt("backup path is not valid text".into()))?;
        self.conn.execute("VACUUM INTO ?1", [target])?;
        remove_if_present(&backup_path(db, KEEP))?;
        for i in (1..KEEP).rev() {
            let from = backup_path(db, i);
            if from.exists() {
                std::fs::rename(&from, backup_path(db, i + 1))?;
            }
        }
        std::fs::rename(&tmp, backup_path(db, 1))?;
        Ok(())
    }

    /// How old the newest backup is (`None` if there is none).
    pub fn backup_age(db: &Path) -> Option<Duration> {
        age_of(&backup_path(db, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vpet_core::PetState;

    fn temp_db(tag: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("vpet-backup-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("vpet.db");
        (dir, db)
    }

    fn named(name: &str) -> PetState {
        let mut s = PetState::new_egg(0, 7);
        s.name = Some(name.into());
        s
    }

    /// Going back to an older VPet must not mistake a newer save for damage
    /// and start a new egg.
    #[test]
    fn a_save_from_a_newer_version_is_refused_and_left_alone() {
        let (dir, db) = temp_db("newer");
        {
            let mut s = Storage::open(&db).unwrap();
            s.save_pet(&named("Kiba")).unwrap();
            s.conn.pragma_update(None, "user_version", 999).unwrap();
        }
        let before = std::fs::read(&db).unwrap();

        assert!(matches!(Storage::open_checked(&db, 42), Err(Error::TooNew)));
        assert_eq!(std::fs::read(&db).unwrap(), before, "the save is untouched");
        assert!(!with_suffix(&db, ".broken-42").exists(), "and not set aside as damaged");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_damaged_database_is_restored_from_the_newest_backup() {
        let (dir, db) = temp_db("restore");
        {
            let mut s = Storage::open(&db).unwrap();
            s.save_pet(&named("Kiba")).unwrap();
            s.backup(&db).unwrap();
            s.save_pet(&named("Mochi")).unwrap();
            s.backup(&db).unwrap();
        }
        assert!(backup_path(&db, 2).exists(), "the older copy is kept too");
        std::fs::write(&db, b"this is not a database at all").unwrap();

        let (s, recovery) = Storage::open_checked(&db, 42).unwrap();
        assert!(matches!(recovery, Recovery::Restored { .. }), "{recovery:?}");
        assert_eq!(s.load_pet().unwrap().unwrap().name.as_deref(), Some("Mochi"));
        assert!(with_suffix(&db, ".broken-42").exists(), "the damaged file is kept, not deleted");
        drop(s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn with_no_usable_backup_it_starts_fresh_and_keeps_the_damage() {
        let (dir, db) = temp_db("fresh");
        std::fs::write(&db, b"garbage").unwrap();
        let (s, recovery) = Storage::open_checked(&db, 7).unwrap();
        assert!(matches!(recovery, Recovery::StartedFresh { .. }));
        assert!(s.load_pet().unwrap().is_none());
        drop(s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_healthy_database_opens_untouched() {
        let (dir, db) = temp_db("healthy");
        Storage::open(&db).unwrap().save_pet(&named("Kiba")).unwrap();
        let (s, recovery) = Storage::open_checked(&db, 1).unwrap();
        assert_eq!(recovery, Recovery::None);
        assert!(s.load_pet().unwrap().is_some());
        drop(s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deleting_all_data_removes_backups_and_damaged_copies() {
        let (dir, db) = temp_db("destroy");
        {
            let mut s = Storage::open(&db).unwrap();
            s.save_pet(&named("Kiba")).unwrap();
            s.backup(&db).unwrap();
            s.backup(&db).unwrap();
        }
        std::fs::write(with_suffix(&db, ".broken-5"), b"old").unwrap();
        Storage::destroy(&db).unwrap();
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
        assert!(left.is_empty(), "left behind: {left:?}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
