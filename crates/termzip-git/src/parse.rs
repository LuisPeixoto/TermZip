//! Reading `git status --porcelain=v2 --branch -z`.
//!
//! Records are separated by NUL. The ones that start with `# ` describe the branch; the others describe a file:
//! `1` an ordinary change, `2` a rename or copy (the old path is the next record), `u` an unmerged file, `?` an
//! untracked one. The output comes from a repository that may not be trusted, so it is bounded and never panics:
//! counts saturate and anything unexpected is skipped.

use serde::Serialize;

/// The most output that is read. Past it the counts are a lower bound, which `truncated` says.
pub const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

/// Where a folder's repository stands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// The branch name; `None` while HEAD is detached.
    pub branch: Option<String>,
    /// HEAD is not on a branch. `commit` says where it is.
    pub detached: bool,
    /// The first 7 characters of the commit HEAD is on; `None` in a repository with no commit yet.
    pub commit: Option<String>,
    /// Commits the branch has that its upstream does not, and the other way round. Both 0 without an upstream.
    pub ahead: u32,
    pub behind: u32,
    pub has_upstream: bool,
    /// Files with a change in the index (staged) and files with a change in the working tree. A file that is
    /// staged and then edited again counts in both.
    pub staged: u32,
    pub modified: u32,
    /// Files that are neither tracked nor ignored.
    pub untracked: u32,
    /// Files in the middle of a merge that has to be resolved.
    pub conflicted: u32,
    /// The output was longer than [`MAX_OUTPUT_BYTES`], so the counts may be higher.
    pub truncated: bool,
}

impl GitStatus {
    /// Whether anything in the working tree differs from HEAD.
    pub fn is_dirty(&self) -> bool {
        self.staged + self.modified + self.untracked + self.conflicted > 0
    }
}

/// `+3` or `-2` as a number; anything else is 0.
fn signed(text: Option<&str>, sign: char) -> u32 {
    text.and_then(|text| text.strip_prefix(sign))
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0)
}

/// Applies one `# key value` record. Returns whether it was the branch name (the header every repository has).
fn read_header(status: &mut GitStatus, header: &[u8]) -> bool {
    let Ok(header) = std::str::from_utf8(header) else {
        return false;
    };
    let (key, value) = header.split_once(' ').unwrap_or((header, ""));
    match key {
        "branch.oid" => {
            status.commit = (value != "(initial)" && !value.is_empty())
                .then(|| value.chars().take(7).collect());
        }
        "branch.head" => {
            if value == "(detached)" {
                status.detached = true;
            } else if !value.is_empty() {
                status.branch = Some(value.to_owned());
            }
            return true;
        }
        "branch.upstream" => status.has_upstream = true,
        "branch.ab" => {
            let mut numbers = value.split(' ');
            status.ahead = signed(numbers.next(), '+');
            status.behind = signed(numbers.next(), '-');
        }
        _ => {}
    }
    false
}

/// A column of the two-letter state of a file: `.` is no change.
fn changed(column: Option<&u8>) -> bool {
    matches!(column, Some(c) if *c != b'.')
}

fn bump(count: &mut u32) {
    *count = count.saturating_add(1);
}

/// The status in the output of `git status --porcelain=v2 --branch -z`, or `None` when there is no branch header
/// (so it is not the output of a repository).
pub fn parse_status(output: &[u8]) -> Option<GitStatus> {
    let mut status = GitStatus {
        truncated: output.len() > MAX_OUTPUT_BYTES,
        ..GitStatus::default()
    };
    let mut saw_head = false;
    let mut records = output[..output.len().min(MAX_OUTPUT_BYTES)].split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix(b"# ") {
            saw_head |= read_header(&mut status, header);
            continue;
        }
        match record.first() {
            Some(b'1' | b'2') => {
                // `1 XY ...`: X is the state in the index, Y in the working tree.
                if changed(record.get(2)) {
                    bump(&mut status.staged);
                }
                if changed(record.get(3)) {
                    bump(&mut status.modified);
                }
                // A rename or copy is followed by its old path, in a record of its own.
                if record[0] == b'2' {
                    records.next();
                }
            }
            Some(b'u') => bump(&mut status.conflicted),
            Some(b'?') => bump(&mut status.untracked),
            _ => {}
        }
    }
    saw_head.then_some(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(records: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for record in records {
            out.extend_from_slice(record.as_bytes());
            out.push(0);
        }
        out
    }

    #[test]
    fn a_branch_with_its_upstream_is_read() {
        let status = parse_status(&text(&[
            "# branch.oid 6004959369aef06e9fbcc3a038cfa6b115184d79",
            "# branch.head main",
            "# branch.upstream origin/main",
            "# branch.ab +2 -3",
        ]))
        .unwrap();

        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.commit.as_deref(), Some("6004959"));
        assert!(status.has_upstream);
        assert_eq!((status.ahead, status.behind), (2, 3));
        assert!(!status.is_dirty());
    }

    #[test]
    fn each_state_of_a_file_is_counted_in_its_column() {
        let status = parse_status(&text(&[
            "# branch.head main",
            "1 .M N... 100644 100644 100644 aaa bbb modified.txt",
            "1 A. N... 000000 100644 100644 000 bbb staged.txt",
            "1 AM N... 000000 100644 100644 000 bbb both.txt",
            "1 D. N... 100644 000000 000000 aaa 000 deleted.txt",
            "? untracked.txt",
            "? another.txt",
            "u UU N... 100644 100644 100644 100644 a b c conflict.txt",
        ]))
        .unwrap();

        assert_eq!(status.staged, 3);
        assert_eq!(status.modified, 2);
        assert_eq!(status.untracked, 2);
        assert_eq!(status.conflicted, 1);
        assert!(status.is_dirty());
    }

    #[test]
    fn a_rename_skips_the_record_with_the_old_path() {
        // The old path could look like anything; it must not be counted as a file of its own.
        let status = parse_status(&text(&[
            "# branch.head main",
            "2 R. N... 100644 100644 100644 aaa bbb R100 new.txt",
            "? looks-like-a-record.txt",
            "1 .M N... 100644 100644 100644 aaa bbb after.txt",
        ]))
        .unwrap();
        // `? looks-like...` was consumed as the old path of the rename.
        assert_eq!(
            (status.staged, status.modified, status.untracked),
            (1, 1, 0)
        );
    }

    #[test]
    fn a_detached_head_has_no_branch_but_a_commit() {
        let status = parse_status(&text(&[
            "# branch.oid 1234567890abcdef",
            "# branch.head (detached)",
        ]))
        .unwrap();

        assert!(status.detached);
        assert_eq!(status.branch, None);
        assert_eq!(status.commit.as_deref(), Some("1234567"));
    }

    #[test]
    fn a_repository_without_a_commit_yet_has_no_commit() {
        let status = parse_status(&text(&[
            "# branch.oid (initial)",
            "# branch.head main",
            "? n.txt",
        ]))
        .unwrap();

        assert_eq!(status.commit, None);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.untracked, 1);
    }

    #[test]
    fn without_an_upstream_there_is_nothing_ahead_or_behind() {
        let status = parse_status(&text(&["# branch.head topic"])).unwrap();

        assert!(!status.has_upstream);
        assert_eq!((status.ahead, status.behind), (0, 0));
    }

    #[test]
    fn output_that_is_not_a_repository_is_none() {
        assert_eq!(parse_status(b""), None);
        assert_eq!(parse_status(b"fatal: not a git repository\n"), None);
        assert_eq!(parse_status(&text(&["? only-a-file"])), None);
    }

    #[test]
    fn a_file_name_that_is_not_utf8_still_counts() {
        let mut output = text(&["# branch.head main"]);
        output.extend_from_slice(b"? \xff\xfe.txt\0");

        assert_eq!(parse_status(&output).unwrap().untracked, 1);
    }

    #[test]
    fn an_enormous_output_is_cut_and_says_so() {
        let mut output = text(&["# branch.head main"]);
        let line = b"? some/long/path/to/a/file.txt\0";
        while output.len() <= MAX_OUTPUT_BYTES {
            output.extend_from_slice(line);
        }

        let status = parse_status(&output).unwrap();

        assert!(status.truncated);
        assert!(status.untracked as usize <= MAX_OUTPUT_BYTES / 2);
    }

    #[test]
    fn hostile_output_never_panics() {
        for output in [
            vec![0u8; 100],
            b"# ".to_vec(),
            b"# \0# \0".to_vec(),
            b"# branch.ab +99999999999999999999 -x\0# branch.head m\0".to_vec(),
            b"1\x002\x00u\x00?\x00".to_vec(),
            b"2 R.\x00".to_vec(),
            vec![0xff; 1000],
            "# branch.head é\u{202e}\0".repeat(1000).into_bytes(),
        ] {
            let _ = parse_status(&output);
        }
    }
}
