//! The layout TermZip reopens on launch: tabs, splits and working directories.
//!
//! It holds structure only. No terminal output, command or environment is ever written to it.
//! Everything read back is bounded and validated, because the file is user-editable input.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage::{set_aside, write_atomic};
use crate::{SettingsError, data_dir};

pub const WORKSPACE_VERSION: u32 = 1;
pub const MAX_FILE_BYTES: u64 = 64 * 1024;
pub const MAX_TABS: usize = 32;
pub const MAX_PANES_PER_TAB: usize = 16;
pub const MAX_DEPTH: usize = 8;
pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;
const MAX_TITLE_CHARS: usize = 64;
const MAX_PATH_CHARS: usize = 4096;
const MAX_ID_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Workspace {
    pub version: u32,
    pub active_tab: usize,
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    /// A name the user gave the tab; `None` follows the shell title.
    #[serde(default)]
    pub title: Option<String>,
    /// Position of the focused pane, counting leaves left to right and top to bottom.
    #[serde(default)]
    pub active_pane: usize,
    pub root: PaneNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PaneNode {
    #[serde(rename_all = "camelCase")]
    Leaf {
        /// Profile id the pane started with.
        profile: Option<String>,
        /// Where the shell was when TermZip last saw it.
        cwd: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Split {
        direction: Direction,
        /// Share of the space the first child takes.
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    /// Side by side.
    Row,
    /// Stacked.
    Column,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            version: WORKSPACE_VERSION,
            active_tab: 0,
            tabs: Vec::new(),
        }
    }
}

impl PaneNode {
    pub fn leaf_count(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { first, second, .. } => 1 + first.depth().max(second.depth()),
        }
    }

    fn cleaned(self) -> Self {
        match self {
            Self::Leaf { profile, cwd } => Self::Leaf {
                profile: profile
                    .map(|id| id.chars().take(MAX_ID_CHARS).collect::<String>())
                    .filter(|id| !id.is_empty()),
                cwd: cwd.filter(|path| {
                    !path.is_empty()
                        && path.chars().count() <= MAX_PATH_CHARS
                        && !path.contains('\0')
                }),
            },
            Self::Split {
                direction,
                ratio,
                first,
                second,
            } => Self::Split {
                direction,
                ratio: if ratio.is_finite() {
                    ratio.clamp(MIN_RATIO, MAX_RATIO)
                } else {
                    0.5
                },
                first: Box::new(first.cleaned()),
                second: Box::new(second.cleaned()),
            },
        }
    }
}

impl Workspace {
    /// Checks the limits and cleans every field. A workspace that breaks a limit is refused as a
    /// whole rather than pruned, because a half-restored layout would only confuse.
    pub fn normalized(mut self) -> Result<Self, String> {
        if self.version != WORKSPACE_VERSION {
            return Err(format!("unsupported workspace version {}", self.version));
        }
        if self.tabs.len() > MAX_TABS {
            return Err(format!("more than {MAX_TABS} tabs"));
        }
        for tab in &self.tabs {
            if tab.root.depth() > MAX_DEPTH {
                return Err(format!("splits nested deeper than {MAX_DEPTH}"));
            }
            if tab.root.leaf_count() > MAX_PANES_PER_TAB {
                return Err(format!("more than {MAX_PANES_PER_TAB} panes in a tab"));
            }
        }
        self.tabs = self
            .tabs
            .into_iter()
            .map(|mut tab| {
                tab.title = tab
                    .title
                    .map(|title| {
                        title
                            .trim()
                            .chars()
                            .take(MAX_TITLE_CHARS)
                            .collect::<String>()
                    })
                    .filter(|title| !title.is_empty());
                tab.root = tab.root.cleaned();
                tab.active_pane = tab.active_pane.min(tab.root.leaf_count().saturating_sub(1));
                tab
            })
            .collect();
        self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
        Ok(self)
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err("the workspace file is too large".to_owned());
        }
        serde_json::from_str::<Self>(text)
            .map_err(|error| error.to_string())?
            .normalized()
    }

    pub fn load() -> Result<Option<Self>, SettingsError> {
        Self::load_from(&workspace_path()?)
    }

    /// `None` when there is nothing to restore: no file, or one that is oversized, malformed or
    /// over a limit (it is then kept as `<name>.invalid`, not deleted or overwritten).
    pub fn load_from(path: &Path) -> Result<Option<Self>, SettingsError> {
        match fs::metadata(path) {
            Ok(meta) if meta.len() > MAX_FILE_BYTES => {
                set_aside(path);
                return Ok(None);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(SettingsError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        }
        let text = fs::read_to_string(path).map_err(|source| SettingsError::Read {
            path: path.to_owned(),
            source,
        })?;
        match Self::from_json(&text) {
            Ok(workspace) => Ok(Some(workspace)),
            Err(_) => {
                set_aside(path);
                Ok(None)
            }
        }
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        self.save_to(&workspace_path()?)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), SettingsError> {
        let workspace = self.clone().normalized().map_err(SettingsError::Invalid)?;
        let json = serde_json::to_vec_pretty(&workspace)
            .map_err(|error| SettingsError::Invalid(error.to_string()))?;
        write_atomic(path, &json)
    }
}

pub fn workspace_path() -> Result<PathBuf, SettingsError> {
    Ok(data_dir()?.join("workspace.json"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn leaf(cwd: &str) -> PaneNode {
        PaneNode::Leaf {
            profile: None,
            cwd: Some(cwd.to_owned()),
        }
    }

    fn split(first: PaneNode, second: PaneNode) -> PaneNode {
        PaneNode::Split {
            direction: Direction::Row,
            ratio: 0.5,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    fn tab(root: PaneNode) -> Tab {
        Tab {
            title: None,
            active_pane: 0,
            root,
        }
    }

    fn workspace(tabs: Vec<Tab>) -> Workspace {
        Workspace {
            tabs,
            ..Workspace::default()
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("termzip-workspace-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A chain of splits `depth` levels deep.
    fn nested(depth: usize) -> PaneNode {
        (1..depth).fold(leaf("/"), |inner, _| split(inner, leaf("/")))
    }

    #[test]
    fn a_workspace_survives_a_round_trip() {
        let original = Workspace {
            active_tab: 1,
            tabs: vec![
                Tab {
                    title: Some("build".to_owned()),
                    active_pane: 1,
                    root: PaneNode::Split {
                        direction: Direction::Column,
                        ratio: 0.3,
                        first: Box::new(leaf("/src")),
                        second: Box::new(PaneNode::Leaf {
                            profile: Some("work".to_owned()),
                            cwd: None,
                        }),
                    },
                },
                tab(leaf("/tmp")),
            ],
            ..Workspace::default()
        };
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(Workspace::from_json(&json).unwrap(), original);
    }

    #[test]
    fn the_file_uses_readable_camel_case_names() {
        let json =
            serde_json::to_string(&workspace(vec![tab(split(leaf("/a"), leaf("/b")))])).unwrap();
        for expected in [
            "\"activeTab\"",
            "\"activePane\"",
            "\"kind\":\"split\"",
            "\"direction\":\"row\"",
            "\"kind\":\"leaf\"",
        ] {
            assert!(json.contains(expected), "{expected} missing in {json}");
        }
    }

    #[test]
    fn limits_refuse_the_whole_workspace() {
        let version = Workspace {
            version: 99,
            ..Workspace::default()
        };
        assert!(version.normalized().unwrap_err().contains("version"));

        let tabs = workspace((0..=MAX_TABS).map(|_| tab(leaf("/"))).collect());
        assert!(tabs.normalized().unwrap_err().contains("tabs"));

        let deep = workspace(vec![tab(nested(MAX_DEPTH + 1))]);
        assert!(deep.normalized().unwrap_err().contains("deeper"));
        assert!(workspace(vec![tab(nested(MAX_DEPTH))]).normalized().is_ok());
    }

    #[test]
    fn too_many_panes_are_refused_even_when_shallow() {
        // 16 + 1 leaves spread over a depth-5 balanced tree.
        fn balanced(leaves: usize) -> PaneNode {
            if leaves == 1 {
                return leaf("/");
            }
            split(balanced(leaves / 2), balanced(leaves - leaves / 2))
        }
        assert!(
            workspace(vec![tab(balanced(MAX_PANES_PER_TAB))])
                .normalized()
                .is_ok()
        );
        let error = workspace(vec![tab(balanced(MAX_PANES_PER_TAB + 1))])
            .normalized()
            .unwrap_err();
        assert!(error.contains("panes"), "{error}");
    }

    #[test]
    fn hostile_values_are_cleaned_not_trusted() {
        let mut root = split(leaf("/ok"), leaf("/evil\0path"));
        if let PaneNode::Split { ratio, .. } = &mut root {
            *ratio = f32::NAN;
        }
        let dirty = Workspace {
            active_tab: 99,
            tabs: vec![Tab {
                title: Some(format!("  {}  ", "t".repeat(500))),
                active_pane: 99,
                root,
            }],
            ..Workspace::default()
        };

        let cleaned = dirty.normalized().unwrap();

        assert_eq!(cleaned.active_tab, 0);
        assert_eq!(cleaned.tabs[0].active_pane, 1, "clamped to the last pane");
        assert_eq!(
            cleaned.tabs[0].title.as_deref().map(|t| t.chars().count()),
            Some(64)
        );
        let PaneNode::Split { ratio, second, .. } = &cleaned.tabs[0].root else {
            panic!("expected a split");
        };
        assert_eq!(*ratio, 0.5);
        assert_eq!(
            **second,
            PaneNode::Leaf {
                profile: None,
                cwd: None
            }
        );
    }

    #[test]
    fn ratios_are_kept_inside_a_usable_range() {
        for (given, expected) in [
            (0.0, MIN_RATIO),
            (1.0, MAX_RATIO),
            (0.4, 0.4),
            (f32::INFINITY, 0.5),
        ] {
            let node = PaneNode::Split {
                direction: Direction::Row,
                ratio: given,
                first: Box::new(leaf("/")),
                second: Box::new(leaf("/")),
            };
            let cleaned = workspace(vec![tab(node)]).normalized().unwrap();
            let PaneNode::Split { ratio, .. } = &cleaned.tabs[0].root else {
                panic!("expected a split");
            };
            assert_eq!(*ratio, expected, "for {given}");
        }
    }

    #[test]
    fn a_missing_file_means_nothing_to_restore() {
        let dir = temp_dir("missing");
        assert_eq!(
            Workspace::load_from(&dir.join("workspace.json")).unwrap(),
            None
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_is_kept_aside_and_ignored() {
        let dir = temp_dir("broken");
        let path = dir.join("workspace.json");
        fs::write(&path, "{ not json").unwrap();

        assert_eq!(Workspace::load_from(&path).unwrap(), None);
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(dir.join("workspace.json.invalid")).unwrap(),
            "{ not json"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_oversized_file_is_never_read() {
        let dir = temp_dir("huge");
        let path = dir.join("workspace.json");
        fs::write(&path, vec![b' '; MAX_FILE_BYTES as usize + 1]).unwrap();

        assert_eq!(Workspace::load_from(&path).unwrap(), None);
        assert!(dir.join("workspace.json.invalid").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_over_a_limit_is_kept_aside_too() {
        let dir = temp_dir("deep");
        let path = dir.join("workspace.json");
        let deep = workspace(vec![tab(nested(MAX_DEPTH + 1))]);
        fs::write(&path, serde_json::to_string(&deep).unwrap()).unwrap();

        assert_eq!(Workspace::load_from(&path).unwrap(), None);
        assert!(dir.join("workspace.json.invalid").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_is_atomic_and_refuses_an_invalid_workspace() {
        let dir = temp_dir("save");
        let path = dir.join("workspace.json");
        let good = workspace(vec![tab(split(leaf("/a"), leaf("/b")))]);

        good.save_to(&path).unwrap();
        assert_eq!(Workspace::load_from(&path).unwrap(), Some(good));
        let files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(files, vec![OsString::from("workspace.json")]);

        let bad = workspace(vec![tab(nested(MAX_DEPTH + 1))]);
        assert!(matches!(bad.save_to(&path), Err(SettingsError::Invalid(_))));
        assert!(
            Workspace::load_from(&path).unwrap().is_some(),
            "the good file must survive"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn absurdly_nested_json_is_an_error_not_a_crash() {
        let depth = 5_000;
        let mut json = String::new();
        for _ in 0..depth {
            json.push_str(r#"{"kind":"split","direction":"row","ratio":0.5,"first":"#);
        }
        json.push_str(r#"{"kind":"leaf"}"#);
        for _ in 0..depth {
            json.push_str(r#","second":{"kind":"leaf"}}"#);
        }
        let text = format!(r#"{{"tabs":[{{"root":{json}}}]}}"#);
        // Far past the parser's recursion limit: an error, never a stack overflow.
        assert!(serde_json::from_str::<Workspace>(&text).is_err());
        assert!(Workspace::from_json(&text).is_err());
    }
}
