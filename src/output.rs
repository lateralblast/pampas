//! Rendering a run's results as markdown, plain text or JSON.

use clap::ValueEnum;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Format {
    /// Plain text: `name:` then the answer
    Text,
    /// `## name` headings (the default)
    #[value(alias = "md")]
    Markdown,
    /// One JSON object with every answer, failures included
    Json,
}

/// One provider's answer, or the reason it failed.
pub struct Entry {
    pub provider: String,
    pub outcome: Result<String, String>,
}

/// Everything a run produced.
pub struct Report {
    pub prompt: String,
    pub entries: Vec<Entry>,
    /// Provider that wrote the consolidated answer, and the answer.
    pub merged: Option<(String, String)>,
    /// Why `--merge` produced no consolidated answer, when it was asked for and failed.
    pub merge_error: Option<String>,
    /// With a merged answer, also show the separate answers (markdown and text only).
    pub show_answers: bool,
}

pub fn render(report: &Report, format: Format) -> String {
    match format {
        Format::Json => render_json(report),
        Format::Markdown => render_sections(report, |name| format!("## {name}")),
        Format::Text => render_sections(report, |name| format!("{name}:")),
    }
}

/// Sections of `heading(name)`, the answer and a blank line. The merged answer comes first; the
/// separate answers follow unless a merged one replaces them.
fn render_sections(report: &Report, heading: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    if let Some((by, text)) = &report.merged {
        out.push_str(&format!("{}\n{text}\n\n", heading(&format!("merged (by {by})"))));
    }
    if report.merged.is_none() || report.show_answers {
        for e in &report.entries {
            if let Ok(text) = &e.outcome {
                out.push_str(&format!("{}\n{text}\n\n", heading(&e.provider)));
            }
        }
    }
    out
}

fn render_json(report: &Report) -> String {
    let answers: Vec<_> = report
        .entries
        .iter()
        .map(|e| match &e.outcome {
            Ok(text) => json!({"provider": e.provider, "ok": true, "answer": text}),
            Err(err) => json!({"provider": e.provider, "ok": false, "error": err}),
        })
        .collect();
    let merged = report.merged.as_ref().map(|(by, text)| json!({"provider": by, "answer": text}));
    let doc = json!({
        "prompt": report.prompt,
        "answers": answers,
        "merged": merged,
        "merge_error": report.merge_error,
    });
    let mut out = serde_json::to_string_pretty(&doc).expect("a JSON value always serializes");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(merged: bool, show_answers: bool) -> Report {
        Report {
            prompt: "q".into(),
            entries: vec![
                Entry { provider: "a".into(), outcome: Ok("one".into()) },
                Entry { provider: "b".into(), outcome: Err("HTTP 429".into()) },
                Entry { provider: "c".into(), outcome: Ok("two".into()) },
            ],
            merged: merged.then(|| ("c".to_string(), "both".to_string())),
            merge_error: None,
            show_answers,
        }
    }

    #[test]
    fn markdown_matches_the_original_output() {
        assert_eq!(render(&report(false, false), Format::Markdown), "## a\none\n\n## c\ntwo\n\n");
    }

    #[test]
    fn text_has_no_markup_and_skips_failures() {
        assert_eq!(render(&report(false, false), Format::Text), "a:\none\n\nc:\ntwo\n\n");
    }

    #[test]
    fn merged_replaces_answers_unless_shown() {
        assert_eq!(render(&report(true, false), Format::Markdown), "## merged (by c)\nboth\n\n");
        assert_eq!(
            render(&report(true, true), Format::Text),
            "merged (by c):\nboth\n\na:\none\n\nc:\ntwo\n\n"
        );
    }

    #[test]
    fn json_has_every_answer_failures_and_the_merge() {
        let v: serde_json::Value = serde_json::from_str(&render(&report(true, false), Format::Json)).unwrap();
        assert_eq!(v["prompt"], "q");
        assert_eq!(v["answers"].as_array().unwrap().len(), 3);
        assert_eq!(v["answers"][0], json!({"provider": "a", "ok": true, "answer": "one"}));
        assert_eq!(v["answers"][1], json!({"provider": "b", "ok": false, "error": "HTTP 429"}));
        assert_eq!(v["merged"], json!({"provider": "c", "answer": "both"}));
        assert!(v["merge_error"].is_null());
        let plain: serde_json::Value = serde_json::from_str(&render(&report(false, false), Format::Json)).unwrap();
        assert!(plain["merged"].is_null());
    }
}
