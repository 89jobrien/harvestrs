use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

/// Sync all project memory files to vault's claude-memory/ directory.
/// One markdown file per project slug: vault/claude-memory/<project-name>.md
pub fn sync_all(projects_root: &Path, vault_root: &Path) -> Result<()> {
    let out_dir = vault_root.join("claude-memory");
    std::fs::create_dir_all(&out_dir)?;

    for entry in WalkDir::new(projects_root)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let slug = entry.file_name().to_string_lossy().to_string();
        let memory_dir = entry.path().join("memory");
        if !memory_dir.exists() {
            continue;
        }

        let files: Vec<_> = WalkDir::new(&memory_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().is_some_and(|x| x == "md")
                    && e.path().file_name().is_some_and(|n| n != "MEMORY.md")
            })
            .collect();

        if files.is_empty() {
            continue;
        }

        // Use the full normalized slug as the project key to guarantee uniqueness across
        // vaults, users, and machines. The slug is the directory name under ~/.claude/projects
        // and is already globally unique for a given Claude installation. Strip only the
        // leading dash so the filename is readable.
        let project_name = slug.trim_start_matches('-').to_string();

        let mut sections = vec![
            format!("# Claude Session Context — {}\n", project_name),
            "> Auto-synced from Claude Code session memory.\n".into(),
            format!("> Source: `~/.claude/projects/{}/memory/`\n", slug),
        ];

        let mut sorted_files: Vec<_> = files.iter().collect();
        sorted_files.sort_by_key(|e| e.path().file_name().unwrap().to_string_lossy().to_string());

        for f in sorted_files {
            let content = std::fs::read_to_string(f.path())?;
            let body = if content.starts_with("---\n") {
                content
                    .splitn(3, "---\n")
                    .nth(2)
                    .unwrap_or(&content)
                    .trim()
                    .to_string()
            } else {
                content.trim().to_string()
            };
            let stem = f.path().file_stem().unwrap().to_string_lossy();
            sections.push(format!("\n---\n\n## {}\n\n{}\n", stem, body));
        }

        let note = sections.join("");
        let out_path = out_dir.join(format!("{}.md", project_name));
        // Only write if content changed to avoid unnecessary vault churn.
        let existing = std::fs::read_to_string(&out_path).unwrap_or_default();
        if existing != note {
            std::fs::write(&out_path, note)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn syncs_memory_files_to_vault() {
        let projects = TempDir::new().unwrap();
        let vault = TempDir::new().unwrap();

        let memory_dir = projects
            .path()
            .join("-Users-joe-dev-minibox")
            .join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(
            memory_dir.join("project_state.md"),
            "---\nname: state\ntype: project\n---\nAll tests passing.",
        )
        .unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "# index").unwrap();

        sync_all(projects.path(), vault.path()).unwrap();

        let out = vault.path().join("claude-memory").join("Users-joe-dev-minibox.md");
        assert!(out.exists(), "expected context note at {}", out.display());
        let content = fs::read_to_string(&out).unwrap();
        assert!(content.contains("All tests passing."));
    }

    #[test]
    fn does_not_overwrite_unchanged_content() {
        let projects = TempDir::new().unwrap();
        let vault = TempDir::new().unwrap();

        let memory_dir = projects
            .path()
            .join("-Users-joe-dev-minibox")
            .join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(
            memory_dir.join("project_state.md"),
            "---\nname: state\ntype: project\n---\nStable content.",
        )
        .unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "# index").unwrap();

        // First sync — creates the file.
        sync_all(projects.path(), vault.path()).unwrap();
        let out = vault.path().join("claude-memory").join("Users-joe-dev-minibox.md");
        let mtime1 = fs::metadata(&out).unwrap().modified().unwrap();

        // Brief sleep to ensure mtime would differ if re-written.
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Second sync — content unchanged, file must NOT be rewritten.
        sync_all(projects.path(), vault.path()).unwrap();
        let mtime2 = fs::metadata(&out).unwrap().modified().unwrap();

        assert_eq!(mtime1, mtime2, "file was rewritten despite no content change");
    }

    #[test]
    fn project_name_uses_two_segments_to_avoid_collision() {
        let projects = TempDir::new().unwrap();
        let vault = TempDir::new().unwrap();

        // Two different slugs that share the same last segment ("minibox").
        for slug in ["-Users-alice-dev-minibox", "-Users-bob-dev-minibox"] {
            let memory_dir = projects.path().join(slug).join("memory");
            fs::create_dir_all(&memory_dir).unwrap();
            fs::write(
                memory_dir.join("state.md"),
                format!("content for {slug}"),
            )
            .unwrap();
        }

        sync_all(projects.path(), vault.path()).unwrap();

        let out_dir = vault.path().join("claude-memory");
        // Both project slugs end in "minibox" but differ in second-to-last segment,
        // so both notes must be written (no collision).
        let count = fs::read_dir(&out_dir).unwrap().count();
        assert_eq!(count, 2, "expected 2 distinct output files, got {count}");
    }

    #[test]
    fn skips_projects_with_no_memory_dir() {
        let projects = TempDir::new().unwrap();
        let vault = TempDir::new().unwrap();
        fs::create_dir(projects.path().join("-Users-joe-dev-empty")).unwrap();

        sync_all(projects.path(), vault.path()).unwrap();

        let out_dir = vault.path().join("claude-memory");
        let count = fs::read_dir(&out_dir).map(|d| d.count()).unwrap_or(0);
        assert_eq!(count, 0);
    }
}
