//! Every tracked Markdown file closes the code fences it opens.
//!
//! An unclosed fence fails nothing: the file still parses, links still resolve, and the text reads
//! fine in an editor. What breaks is the rendering — everything after the fence becomes one code
//! block, headings included. A nested example is the usual cause: an inner ```` ``` ```` closes a
//! three-backtick outer fence early, and the outer one's own closing line then opens a block that
//! runs to the end of the document. The specification lost a section heading that way.
//!
//! Fences follow CommonMark: an opening run of three or more backticks or tildes (indented at most
//! three spaces) is closed only by a run of the same character at least as long, with nothing
//! after it. A four-backtick fence around a three-backtick example is therefore fine.
//!
//! Like the public-text guard, this lives in the CLI crate's tests only because they already resolve
//! the workspace root.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .to_path_buf()
}

fn tracked_markdown(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-files", "-z", "--", "*.md"])
        .current_dir(root)
        .output()
        .expect("failed to run `git ls-files`");
    assert!(
        output.status.success(),
        "`git ls-files` failed in {}: {}",
        root.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("non-UTF-8 path")
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A fence line's character and length, and whether anything follows the run.
fn fence(line: &str) -> Option<(char, usize, bool)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let run = rest.chars().take_while(|c| *c == ch).count();
    if run < 3 {
        return None;
    }
    let after = rest[run..].trim();
    // A backtick fence's info string may not contain a backtick — such a line is inline code.
    if ch == '`' && after.contains('`') {
        return None;
    }
    Some((ch, run, !after.is_empty()))
}

/// The 1-based line of a fence that is never closed, if any.
fn unclosed_fence(text: &str) -> Option<usize> {
    let mut open: Option<(char, usize, usize)> = None;
    for (i, line) in text.lines().enumerate() {
        let Some((ch, run, has_info)) = fence(line) else {
            continue;
        };
        match open {
            None => open = Some((ch, run, i + 1)),
            Some((open_ch, open_run, _)) if ch == open_ch && run >= open_run && !has_info => {
                open = None
            }
            Some(_) => {}
        }
    }
    open.map(|(_, _, line)| line)
}

#[test]
fn every_tracked_markdown_file_closes_its_code_fences() {
    let root = workspace_root();
    let files = tracked_markdown(&root);
    assert!(
        files.iter().any(|f| f == "docs/specification.md"),
        "the scan did not see the specification — `git ls-files` is not listing what it should"
    );

    let unclosed: Vec<String> = files
        .iter()
        .filter_map(|f| {
            let text = std::fs::read_to_string(root.join(f)).ok()?;
            unclosed_fence(&text).map(|line| format!("{f}:{line}"))
        })
        .collect();

    assert!(
        unclosed.is_empty(),
        "code fence opened and never closed — everything after it renders as one code block, \
         headings included. Close it, or give an outer fence more backticks than the one it \
         contains:\n  {}",
        unclosed.join("\n  ")
    );
}

#[test]
fn the_check_tells_a_closed_nest_from_an_early_close() {
    // Four backticks around three: the inner closing line does not close the outer fence.
    assert_eq!(
        unclosed_fence("````markdown\n```sql\nx\n```\n````\n# Next\n"),
        None
    );
    // Three around three: the inner closing line closes the outer fence, and the outer's own
    // closing line then opens a block that never ends.
    assert_eq!(
        unclosed_fence("```markdown\n```sql\nx\n```\n```\n# Next\n"),
        Some(5)
    );
    // A closing run may be longer; a tilde fence is not closed by backticks.
    assert_eq!(unclosed_fence("```\nx\n`````\n"), None);
    assert_eq!(unclosed_fence("~~~\nx\n```\n"), Some(1));
    // Inline code is not a fence.
    assert_eq!(unclosed_fence("Use ```x``` inline.\n"), None);
}
