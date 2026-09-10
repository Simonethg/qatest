use std::path::{Path, PathBuf};

pub fn git_name_and_branch(cwd: &Path) -> (String, String) {
    let name = cwd
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "space".into());
    let branch = read_git_branch(cwd).unwrap_or_else(|| "no-git".into());
    (name, branch)
}

fn read_git_branch(cwd: &Path) -> Option<String> {
    let mut dir = cwd.to_path_buf();
    loop {
        let head = dir.join(".git").join("HEAD");
        if head.is_file() {
            let raw = std::fs::read_to_string(head).ok()?;
            return parse_head(&raw);
        }
        if dir.join(".git").is_file() {
            if let Some(gitdir) = gitdir_from_file(&dir.join(".git")) {
                let head = gitdir.join("HEAD");
                let raw = std::fs::read_to_string(head).ok()?;
                return parse_head(&raw);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn gitdir_from_file(gitfile: &Path) -> Option<PathBuf> {
    let raw = std::fs::read_to_string(gitfile).ok()?;
    raw.strip_prefix("gitdir:")
        .map(|s| s.trim())
        .map(PathBuf::from)
}

fn parse_head(raw: &str) -> Option<String> {
    let t = raw.trim();
    if let Some(r) = t.strip_prefix("ref: refs/heads/") {
        return Some(r.trim().to_string());
    }
    Some(t.chars().take(7).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_ref() {
        assert_eq!(
            parse_head("ref: refs/heads/main\n").as_deref(),
            Some("main")
        );
    }
}
