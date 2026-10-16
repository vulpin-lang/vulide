//! User configuration — `~/.config/vulide/config.toml`.
//!
//! Mirrors the keys of the Python IDE's `DEFAULT_SETTINGS` that make sense for a
//! terminal build. Anything the file omits keeps its default; an unreadable or
//! malformed file falls back to defaults with a warning on the status bar (the
//! editor never refuses to start over a bad config).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Name of the active theme (matches a `Theme::name`).
    pub theme: String,
    pub tab_width: usize,
    /// Stored + togg(l)eable; soft-wrap rendering is a later phase.
    pub word_wrap: bool,
    pub auto_close_brackets: bool,
    pub auto_indent: bool,
    pub show_autocomplete: bool,
    pub show_line_numbers: bool,
    /// Show the structure/outline sidebar (`F7`).
    pub show_algo: bool,
    /// Show the file-tree sidebar (`F2`).
    pub show_files: bool,
    /// Capture the mouse so the status-bar ▶ button and click-to-focus work.
    /// Turn off to get the terminal's own text selection back everywhere.
    pub mouse: bool,
    pub auto_save: bool,
    pub recent_files: Vec<PathBuf>,
    pub recent_files_limit: usize,
    /// Directories opened as a project (`Ctrl+O` on a folder, or the `F8`
    /// Projects picker) — most-recent first.
    pub recent_projects: Vec<PathBuf>,
    pub recent_projects_limit: usize,
    /// Reopen the previous session's files on launch (when no file is given).
    /// The file list itself lives in `$XDG_STATE_HOME/vulide/session.toml`
    /// (see `crate::session`), not here.
    pub restore_session: bool,
    /// Explicit interpreter path for Phase 4's run console (`""` = autodetect).
    pub vulpin_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "Dark (Catppuccin Mocha)".to_string(),
            tab_width: 4,
            word_wrap: false,
            auto_close_brackets: true,
            auto_indent: true,
            show_autocomplete: true,
            show_line_numbers: true,
            show_algo: false,
            show_files: false,
            mouse: true,
            auto_save: false,
            recent_files: Vec::new(),
            recent_files_limit: 10,
            recent_projects: Vec::new(),
            recent_projects_limit: 10,
            restore_session: true,
            vulpin_path: String::new(),
        }
    }
}

impl Config {
    /// `$XDG_CONFIG_HOME/vulide/config.toml`, else `$HOME/.config/vulide/…`.
    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("vulide").join("config.toml"))
    }

    /// Load the config. The second field is a warning to surface if the file
    /// exists but could not be used.
    pub fn load() -> (Config, Option<String>) {
        let Some(path) = Self::path() else {
            return (Config::default(), None);
        };
        match std::fs::read_to_string(&path) {
            Ok(src) => match toml::from_str::<Config>(&src) {
                Ok(mut cfg) => {
                    cfg.clamp();
                    (cfg, None)
                }
                Err(e) => (
                    Config::default(),
                    Some(format!(
                        "config: {} — using defaults",
                        first_line(&e.to_string())
                    )),
                ),
            },
            // Missing file is the normal first-run case, not a problem.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
            Err(e) => (
                Config::default(),
                Some(format!("config unreadable: {e} — using defaults")),
            ),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("no HOME/XDG_CONFIG_HOME to save config under")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let body = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Record `path` as the most-recently-opened file.
    pub fn push_recent(&mut self, path: &Path) {
        let path = path.to_path_buf();
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(self.recent_files_limit.max(1));
    }

    /// Record `path` as the most-recently-opened project directory.
    pub fn push_recent_project(&mut self, path: &Path) {
        let path = path.to_path_buf();
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects
            .truncate(self.recent_projects_limit.max(1));
    }

    fn clamp(&mut self) {
        self.tab_width = self.tab_width.clamp(1, 16);
        self.recent_files_limit = self.recent_files_limit.clamp(1, 100);
        self.recent_files.truncate(self.recent_files_limit);
        self.recent_projects_limit = self.recent_projects_limit.clamp(1, 100);
        self.recent_projects.truncate(self.recent_projects_limit);
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_keys_use_defaults() {
        let cfg: Config = toml::from_str("theme = \"Nord\"\ntab_width = 2").unwrap();
        assert_eq!(cfg.theme, "Nord");
        assert_eq!(cfg.tab_width, 2);
        assert!(cfg.auto_close_brackets); // default preserved
        assert_eq!(cfg.recent_files_limit, 10);
    }

    #[test]
    fn roundtrips_through_toml() {
        let mut cfg = Config {
            theme: "Monokai".into(),
            ..Config::default()
        };
        cfg.push_recent(Path::new("/tmp/a.vul"));
        let back: Config = toml::from_str(&toml::to_string_pretty(&cfg).unwrap()).unwrap();
        assert_eq!(back.theme, "Monokai");
        assert_eq!(back.recent_files, vec![PathBuf::from("/tmp/a.vul")]);
    }

    #[test]
    fn push_recent_dedupes_and_caps() {
        let mut cfg = Config {
            recent_files_limit: 3,
            ..Config::default()
        };
        for p in ["/a", "/b", "/c", "/a", "/d"] {
            cfg.push_recent(Path::new(p));
        }
        assert_eq!(
            cfg.recent_files,
            vec![
                PathBuf::from("/d"),
                PathBuf::from("/a"),
                PathBuf::from("/c")
            ]
        );
    }

    #[test]
    fn push_recent_project_dedupes_and_caps() {
        let mut cfg = Config {
            recent_projects_limit: 2,
            ..Config::default()
        };
        for p in ["/proj-a", "/proj-b", "/proj-a", "/proj-c"] {
            cfg.push_recent_project(Path::new(p));
        }
        assert_eq!(
            cfg.recent_projects,
            vec![PathBuf::from("/proj-c"), PathBuf::from("/proj-a")]
        );
    }
}
