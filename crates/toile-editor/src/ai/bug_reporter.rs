//! Bug reporter — creates GitHub Issues for Toile engine/editor bugs via `gh` CLI.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Tracks reported issues to avoid duplicates within a session.
#[derive(Default)]
pub struct BugReporter {
    reported_hashes: HashSet<u64>,
    issues_this_session: u32,
}

const MAX_ISSUES_PER_SESSION: u32 = 5;

impl BugReporter {
    /// Create a GitHub Issue via `gh` CLI. Returns the issue URL or an error.
    pub fn report(
        &mut self,
        repo: &str,
        severity: &str,
        title: &str,
        description: &str,
        component: &str,
        logs: &[String],
    ) -> Result<String, String> {
        // Rate limit
        if self.issues_this_session >= MAX_ISSUES_PER_SESSION {
            return Err(format!(
                "Rate limit: already created {} issues this session (max {})",
                self.issues_this_session, MAX_ISSUES_PER_SESSION
            ));
        }

        // Dedup by title + component hash
        let hash = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            title.hash(&mut hasher);
            component.hash(&mut hasher);
            hasher.finish()
        };
        if self.reported_hashes.contains(&hash) {
            return Err("Duplicate: a similar issue was already reported this session".into());
        }

        // Check gh is available
        if std::process::Command::new("gh")
            .arg("--version")
            .output()
            .is_err()
        {
            return Err("gh CLI not found. Install it: https://cli.github.com/".into());
        }

        // Check for existing similar issue
        let search_output = std::process::Command::new("gh")
            .args([
                "issue", "list",
                "--repo", repo,
                "--search", title,
                "--limit", "5",
                "--json", "number,title",
            ])
            .output();

        if let Ok(output) = search_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(issues) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                for issue in &issues {
                    if let Some(existing_title) = issue.get("title").and_then(|v| v.as_str()) {
                        if titles_similar(existing_title, title) {
                            let num = issue.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                            self.reported_hashes.insert(hash);
                            return Err(format!(
                                "Similar issue already exists: #{num} — {existing_title}"
                            ));
                        }
                    }
                }
            }
        }

        // Build issue body
        let logs_section = if logs.is_empty() {
            String::new()
        } else {
            format!(
                "\n## Relevant logs\n\n```\n{}\n```\n",
                logs.join("\n")
            )
        };

        let body = format!(
            "## Auto-detected by Toile Editor\n\n\
             **Component:** `{component}`\n\
             **Severity:** {severity}\n\
             **Source:** AI Copilot\n\n\
             ## Description\n\n\
             {description}\n\
             {logs_section}\n\
             ## System info\n\n\
             - Toile version: {}\n\
             - OS: {} {}\n\n\
             ---\n\
             *This issue was automatically created by the Toile Editor bug reporter.*",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );

        // Create the issue
        let label = match severity {
            "crash" => "crash",
            "perf" => "performance",
            "enhancement" => "enhancement",
            _ => "bug",
        };

        // Ensure labels exist (ignore errors if they already exist)
        let labels = vec![
            label.to_string(),
            "auto-detected".to_string(),
            format!("component:{component}"),
        ];
        for l in &labels {
            let _ = std::process::Command::new("gh")
                .args(["label", "create", l, "--repo", repo, "--force"])
                .output();
        }

        let mut args = vec![
            "issue".to_string(), "create".to_string(),
            "--repo".to_string(), repo.to_string(),
            "--title".to_string(), title.to_string(),
            "--body".to_string(), body.clone(),
        ];
        for l in &labels {
            args.push("--label".to_string());
            args.push(l.clone());
        }

        let output = std::process::Command::new("gh")
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to run gh: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("gh issue create failed: {stderr}"));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.reported_hashes.insert(hash);
        self.issues_this_session += 1;

        Ok(url)
    }
}

/// Simple similarity check: lowercase titles share enough words.
fn titles_similar(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_words: HashSet<String> = a_lower.split_whitespace().map(String::from).collect();
    let b_words: HashSet<String> = b_lower.split_whitespace().map(String::from).collect();
    if a_words.is_empty() || b_words.is_empty() {
        return false;
    }
    let intersection = a_words.intersection(&b_words).count();
    let min_len = a_words.len().min(b_words.len());
    intersection * 100 / min_len >= 60
}
