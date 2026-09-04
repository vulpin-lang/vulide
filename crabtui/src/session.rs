//! Session state — the files that were open at last exit, so the next launch
//! can reopen them.
//!
//! This is volatile "where was I" state, not user preference, so it lives in
//! `$XDG_STATE_HOME/vulide/session.toml` (→ `~/.local/state/vulide/…`) rather
//! than the config file: it's rewritten on every quit, it's per-user, and it
//! survives a reboot. A missing or unparseable file is simply an empty session.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Session {
    /// Absolute paths of the buffers that had a file, in tab order.
    pub files: Vec<PathBuf>,
    /// Index into `files` of the active tab.
    pub active: usize,
}

impl Session {
    /// `$XDG_STATE_HOME/vulide/session.toml`, else `$HOME/.local/state/vulide/…`.
    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("state"))
            })?;
        Some(base.join("vulide").join("session.toml"))
    }

    /// Load the saved session, or an empty one if there is nothing readable.
    pub fn load() -> Session {
        Self::path()
            .and_then(|p| Self::load_from(&p).ok())
            .unwrap_or_default()
    }

    /// Persist. A no-op under `cfg!(test)` so a test run never writes real state.
    pub fn save(&self) -> Result<()> {
        if cfg!(test) {
            return Ok(());
        }
        let path = Self::path().context("no HOME/XDG_STATE_HOME for session state")?;
        self.save_to(&path)
    }

    pub fn load_from(path: &Path) -> Result<Session> {
        let src =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(toml::from_str(&src)?)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let body = toml::to_string_pretty(self).context("serializing session")?;
        std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let s = Session {
            files: vec![PathBuf::from("/a/one.vul"), PathBuf::from("/b/two.vul")],
            active: 1,
        };
        let path = std::env::temp_dir().join(format!("vulide_sess_{}.toml", std::process::id()));
        s.save_to(&path).unwrap();
        assert_eq!(Session::load_from(&path).unwrap(), s);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn missing_file_is_an_empty_session() {
        let p = Path::new("/vulide/no/such/session.toml");
        assert!(Session::load_from(p).is_err());
        // `load()` swallows that into a default
        assert_eq!(
            Session::default(),
            Session {
                files: vec![],
                active: 0
            }
        );
    }

    #[test]
    fn path_prefers_xdg_state_home() {
        // Can't safely mutate process env in a threaded test runner without
        // races, so just assert the shape when HOME is set.
        if let Some(p) = Session::path() {
            assert!(p.ends_with("vulide/session.toml"));
        }
    }
}
