use crate::claims::{parse, Behaviour};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Relation {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub target: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Doc {
    pub id: String,
    #[serde(default)]
    pub path: String,
    #[serde(rename = "type", default)]
    pub doc_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub governs: Vec<String>,
    #[serde(default)]
    pub related: Vec<Relation>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub content_hash: String,
}

impl Doc {
    pub fn fold_targets(&self) -> Vec<&str> {
        self.related
            .iter()
            .filter(|r| r.rel_type == "folds-into")
            .map(|r| r.target.as_str())
            .collect()
    }

    pub fn behaviour(&self, root: &Path) -> Behaviour {
        parse(&self.body).at_file_line(self.body_line_offset(root))
    }

    fn body_line_offset(&self, root: &Path) -> usize {
        let Ok(text) = std::fs::read_to_string(root.join(&self.path)) else {
            return 0;
        };
        let first = self.body.trim_start_matches('\n');
        let leading = self.body.len() - first.len();
        let Some(frontmatter_end) = text.find("\n---").map(|i| i + 4) else {
            return 0;
        };
        let Some(at) = text[frontmatter_end..].find(first) else {
            return 0;
        };
        let newlines_before = text[..frontmatter_end + at].matches('\n').count();
        newlines_before.saturating_sub(leading)
    }

    pub fn is_named(&self, reference: &str) -> bool {
        self.id == reference || self.path == reference
    }
}

#[derive(Debug, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Payload {
    #[serde(default)]
    pub transition: Option<Transition>,
    #[serde(default)]
    pub documents: Vec<Doc>,
    #[serde(default)]
    pub context: Vec<Doc>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub message: String,
}

impl Finding {
    pub fn warning(id: &str, line: Option<usize>, message: impl Into<String>) -> Self {
        Self::new("warning", id, line, message)
    }

    pub fn error(id: &str, line: Option<usize>, message: impl Into<String>) -> Self {
        Self::new("error", id, line, message)
    }

    fn new(
        severity: &'static str,
        id: &str,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            id: Some(id.to_string()),
            line,
            message: message.into(),
        }
    }

    pub fn bare_error(message: impl Into<String>) -> Self {
        Self {
            severity: "error",
            id: None,
            line: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Update {
    pub id: String,
    pub hash: String,
    pub body: String,
}

#[derive(Debug, Serialize, Default)]
pub struct Output {
    pub findings: Vec<Finding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub updates: Vec<Update>,
}

impl Output {
    pub fn findings(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            updates: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct StopInput {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub stop_hook_active: bool,
}
