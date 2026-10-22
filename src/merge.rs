//! Consolidating several providers' answers into one, by asking one of them to synthesize.

use anyhow::anyhow;

/// The prompt that asks a model to merge `answers` (provider name, text) to `question`.
pub fn synthesis_prompt(question: &str, answers: &[(&str, &str)]) -> String {
    let mut out = String::from(
        "Below is a question and answers to it from several AI models. Write one consolidated answer.\n\
         - State what the answers agree on once.\n\
         - Where they disagree, say so explicitly and name which models said what.\n\
         - Do not add facts that none of the answers contain.\n\
         - When you attribute a point, use the model names given.\n\n",
    );
    out.push_str(&format!("Question:\n{question}\n"));
    for (name, text) in answers {
        out.push_str(&format!("\n=== Answer from {name} ===\n{}\n", text.trim()));
    }
    out
}

/// The provider that synthesizes: `requested` (from `--synthesizer`) if given, else the first of
/// `active` marked `designated` in the config, else the first active provider.
pub fn pick_synthesizer<'a>(
    active: &'a [String],
    designated: &[String],
    requested: Option<&str>,
) -> anyhow::Result<&'a str> {
    if let Some(name) = requested {
        return active.iter().map(String::as_str).find(|n| *n == name).ok_or_else(|| {
            anyhow!("synthesizer `{name}` is not a running provider (running: {})", active.join(", "))
        });
    }
    active
        .iter()
        .find(|n| designated.contains(n))
        .or_else(|| active.first())
        .map(String::as_str)
        .ok_or_else(|| anyhow!("no provider available to synthesize"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prompt_has_question_and_every_answer() {
        let p = synthesis_prompt("why?", &[("groq", " because \n"), ("apple", "no idea")]);
        assert!(p.contains("Question:\nwhy?\n"));
        assert!(p.contains("=== Answer from groq ===\nbecause\n"));
        assert!(p.contains("=== Answer from apple ===\nno idea\n"));
        assert!(p.find("groq").unwrap() < p.rfind("apple").unwrap());
    }

    #[test]
    fn synthesizer_order_is_flag_then_designated_then_first() {
        let active = names(&["a", "b", "c"]);
        assert_eq!(pick_synthesizer(&active, &names(&["c"]), Some("b")).unwrap(), "b");
        assert_eq!(pick_synthesizer(&active, &names(&["c", "b"]), None).unwrap(), "b");
        assert_eq!(pick_synthesizer(&active, &[], None).unwrap(), "a");
    }

    #[test]
    fn a_synthesizer_that_is_not_running_is_an_error() {
        let active = names(&["a"]);
        let err = pick_synthesizer(&active, &[], Some("z")).unwrap_err();
        assert!(format!("{err:#}").contains("`z` is not a running provider"));
        assert!(pick_synthesizer(&[], &[], None).is_err());
    }
}
