//! The studio's settings, kept as JSON in the app's configuration folder, and the workspace they
//! point at: the folder whose `jobs/` the job builder saves to and whose `runs/` runs go to.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What the user can set. Unknown or missing fields take their defaults, so older and newer
/// settings files both load.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The studio's own themes, `"system"` (dark or light, as the system is), `"dark"` or
    /// `"light"`; or a daisyUI theme by its name (`"nord"`, `"dracula"`, ...), with
    /// `"daisyui-light"` and `"daisyui-dark"` for daisyUI's own light and dark. The window
    /// reads it (`studio/src/lib/themes.ts`), and takes a name it doesn't know as `"system"`.
    pub theme: String,
    /// The workspace, if chosen; see [`workspace`].
    pub workspace: Option<String>,
    /// Look for a new release when the window opens.
    pub check_updates: bool,
    /// Show the tips on each page.
    pub hints: bool,
    /// The welcome tour was seen.
    pub tour_done: bool,
    /// The tips the user closed.
    pub dismissed: Vec<String>,
    /// The view a run opens in: `"3d"` or `"2d"`.
    pub view: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            theme: "system".into(),
            workspace: None,
            check_updates: true,
            hints: true,
            tour_done: false,
            dismissed: Vec::new(),
            view: "3d".into(),
        }
    }
}

impl Settings {
    /// The settings in `file`, or the defaults when there are none or they don't parse.
    pub fn load(file: &Path) -> Settings {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Writes the settings to `file`, creating its folder.
    pub fn save(&self, file: &Path) -> Result<(), String> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(file, json).map_err(|e| format!("{}: {e}", file.display()))
    }
}

/// The workspace: the one chosen in the settings; else the folder the program was started in,
/// when it has a `jobs/` or `runs/` (a checkout of the repository, or a project's folder); else
/// `photonoxide` in the user's documents.
pub fn workspace(settings: &Settings, started_in: &Path, documents: Option<&Path>) -> PathBuf {
    if let Some(chosen) = settings.workspace.as_deref().filter(|w| !w.is_empty()) {
        return PathBuf::from(chosen);
    }
    if started_in.join("jobs").is_dir() || started_in.join("runs").is_dir() {
        return started_in.to_path_buf();
    }
    documents.map_or_else(|| started_in.to_path_buf(), |d| d.join("photonoxide"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_tolerate_missing_and_unknown_fields() {
        let dir = std::env::temp_dir().join(format!("photonoxide-settings-{}", std::process::id()));
        let file = dir.join("deeper").join("settings.json");
        assert_eq!(Settings::load(&file), Settings::default());
        let s = Settings {
            theme: "light".into(),
            dismissed: vec!["home".into()],
            ..Settings::default()
        };
        s.save(&file).unwrap();
        assert_eq!(Settings::load(&file), s);
        std::fs::write(&file, r#"{"theme":"dark","from_the_future":1}"#).unwrap();
        let old = Settings::load(&file);
        assert_eq!(old.theme, "dark");
        assert!(old.check_updates && old.hints);
        std::fs::write(&file, r#"{"theme":"nord"}"#).unwrap();
        assert_eq!(Settings::load(&file).theme, "nord");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_workspace_is_the_chosen_one_then_a_project_then_documents() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let docs = Path::new("/home/someone/Documents");
        let mut s = Settings::default();
        assert_eq!(workspace(&s, &repo, Some(docs)), repo);
        let elsewhere = std::env::temp_dir().join("photonoxide-no-such-project");
        assert_eq!(
            workspace(&s, &elsewhere, Some(docs)),
            docs.join("photonoxide")
        );
        s.workspace = Some("/data/photonics".into());
        assert_eq!(
            workspace(&s, &repo, Some(docs)),
            PathBuf::from("/data/photonics")
        );
    }
}
