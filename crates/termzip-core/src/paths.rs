//! Folders written by a person: `~` for the home folder, and whether a path is inside one.

use std::path::{Path, PathBuf};

/// `~` and `~/work` start at the home folder; anything else is taken as written.
pub fn expand_home(folder: &str) -> PathBuf {
    if let Some(rest) = folder.strip_prefix('~')
        && (rest.is_empty() || rest.starts_with(['/', '\\']))
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest.trim_start_matches(['/', '\\']));
    }
    Path::new(folder).to_path_buf()
}

/// How deep the folder is when it contains `path` (the folder itself counts), or `None` when it does not.
/// The comparison is by whole path components, so `/srv/app` does not contain `/srv/application`. A deeper
/// folder is a more specific one.
pub fn depth_if_inside(folder: &str, path: &str) -> Option<usize> {
    let folder = expand_home(folder);
    Path::new(path)
        .starts_with(&folder)
        .then(|| folder.components().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_inside_its_folder_and_below() {
        assert!(depth_if_inside("/srv/app", "/srv/app").is_some());
        assert!(depth_if_inside("/srv/app", "/srv/app/src/deep").is_some());
    }

    #[test]
    fn a_path_is_not_inside_a_folder_with_a_longer_name_or_a_parent() {
        assert_eq!(depth_if_inside("/srv/app", "/srv/application"), None);
        assert_eq!(depth_if_inside("/srv/app", "/srv"), None);
        assert_eq!(depth_if_inside("/srv/app", "relative"), None);
    }

    #[test]
    fn a_deeper_folder_has_a_greater_depth() {
        let shallow = depth_if_inside("/srv", "/srv/app/x").unwrap();
        let deep = depth_if_inside("/srv/app", "/srv/app/x").unwrap();
        assert!(deep > shallow);
    }

    #[test]
    fn the_home_folder_is_expanded() {
        let Some(home) = dirs::home_dir() else { return };
        assert_eq!(expand_home("~"), home);
        assert_eq!(expand_home("~/work"), home.join("work"));
        // `~user` is not supported and is taken as a plain name.
        assert_eq!(expand_home("~someone"), Path::new("~someone"));
        let inside = home.join("work").join("x");
        assert!(depth_if_inside("~/work", inside.to_str().unwrap()).is_some());
        assert_eq!(
            depth_if_inside("~/work", home.join("other").to_str().unwrap()),
            None
        );
    }
}
