use crate::claims::{is_behaviour_part, parse, target_of, Claim, BEHAVIOUR_PART};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Relation {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub target: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Part {
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub body: String,
}

pub fn tests_or_default(attributes: &Attributes) -> Vec<String> {
    if !attributes.tests.is_empty() {
        return attributes.tests.clone();
    }
    DEFAULT_TESTS.iter().map(|g| g.to_string()).collect()
}

pub struct BehaviourPart<'a> {
    pub name: &'a str,
    pub target: Option<&'a str>,
    pub claims: Vec<Claim>,
}

const DEFAULT_TESTS: [&str; 4] = ["**/tests/**", "**/*_test.*", "**/*.test.*", "**/*.spec.*"];

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Attributes {
    #[serde(default)]
    pub tests: Vec<String>,
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
    pub parts: Vec<Part>,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub attributes: Attributes,
    #[serde(skip)]
    pub from_store: bool,
}

impl Doc {
    pub fn tests_globs(&self) -> Vec<String> {
        tests_or_default(&self.attributes)
    }

    pub fn fold_targets(&self) -> Vec<&str> {
        self.related
            .iter()
            .filter(|r| r.rel_type == "folds-into")
            .map(|r| r.target.as_str())
            .collect()
    }

    pub fn part(&self, name: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.name == name)
    }

    pub fn part_path(&self, name: &str) -> String {
        match self.part(name).filter(|p| !p.path.is_empty()) {
            Some(part) => part.path.clone(),
            None => Path::new(&self.path)
                .with_file_name(format!("{name}.md"))
                .to_string_lossy()
                .into_owned(),
        }
    }

    pub fn spec_claims(&self) -> Vec<Claim> {
        self.part(BEHAVIOUR_PART)
            .map(|p| parse(&p.body).claims)
            .unwrap_or_default()
    }

    pub fn index_claims(&self, root: &Path) -> Vec<Claim> {
        let offset = self.body_line_offset(root);
        let mut claims = parse(&self.body).claims;
        claims.iter_mut().for_each(|c| c.line += offset);
        claims
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

    pub fn behaviour_parts(&self) -> Vec<BehaviourPart<'_>> {
        self.parts
            .iter()
            .filter(|p| is_behaviour_part(&p.name))
            .map(|p| BehaviourPart {
                name: &p.name,
                target: target_of(&p.name),
                claims: parse(&p.body).claims,
            })
            .collect()
    }

    pub fn is_named(&self, reference: &str) -> bool {
        self.id == reference || self.path == reference
    }
}

#[derive(Debug, Deserialize)]
pub struct Transition {
    pub from: String,
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
    pub part: Option<String>,
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
            part: None,
            line,
            message: message.into(),
        }
    }

    pub fn bare_error(message: impl Into<String>) -> Self {
        Self {
            severity: "error",
            id: None,
            part: None,
            line: None,
            message: message.into(),
        }
    }

    pub fn in_part(mut self, part: &str) -> Self {
        self.part = Some(part.to_string());
        self
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct Update {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
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
