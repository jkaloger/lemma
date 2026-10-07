use crate::annotate::frontmatter_len;
use crate::claims::{self, heading_level, live_lines};
use crate::payload::{Doc, CLAIMS_PART, INDEX_PART, TASKS_PART};
use crate::plan;
use crate::review::PARTS;
use crate::tasks::{self, Task};
use serde_json::{json, Value};
use std::path::Path;

fn object(properties: Value) -> Value {
    let required: Vec<&String> = properties
        .as_object()
        .into_iter()
        .flat_map(|p| p.keys())
        .collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn array_of(items: Value) -> Value {
    json!({"type": "array", "items": items})
}

fn ids_array(ids: &[String]) -> Value {
    match ids.is_empty() {
        true => json!({"type": "array", "items": {"type": "string"}, "maxItems": 0}),
        false => array_of(json!({"enum": ids})),
    }
}

fn live_claim_ids(claims_text: &str) -> Vec<String> {
    claims::parse(claims_text)
        .claims
        .into_iter()
        .filter(|c| !c.struck)
        .map(|c| c.id)
        .collect()
}

fn push_unique(anchors: &mut Vec<String>, anchor: String) {
    if !anchor.is_empty() && !anchors.contains(&anchor) {
        anchors.push(anchor);
    }
}

fn claim_anchors(body: &str) -> Vec<String> {
    let ids = claims::parse(body).claims.into_iter().map(|c| c.id);
    let plain = claims::bullets(body)
        .into_iter()
        .filter(|b| b.id.is_none())
        .map(|b| b.text);
    ids.chain(plain).collect()
}

/// Every anchor `annotate::annotate` accepts for the part's current file.
fn anchors(part: &str, file: &str) -> Vec<String> {
    let front = match part {
        INDEX_PART => frontmatter_len(file),
        _ => 0,
    };
    let body = &file[front..];
    let by_part = match part {
        CLAIMS_PART => claim_anchors(body),
        TASKS_PART => tasks::parse(body).tasks.into_iter().map(|t| t.id).collect(),
        _ => Vec::new(),
    };
    let mut found = vec![part.to_string()];
    by_part.into_iter().for_each(|a| push_unique(&mut found, a));
    for (_, line) in live_lines(body) {
        if let Some((_, name)) = heading_level(line) {
            push_unique(&mut found, name.to_string());
        }
    }
    found
}

fn part_finding(part: &str, file: &str) -> Value {
    object(json!({
        "part": {"const": part},
        "anchor": {"enum": anchors(part, file)},
        "text": {"type": "string"},
    }))
}

pub fn review_doc(root: &Path, delta: &Doc) -> Value {
    let findings: Vec<Value> = PARTS
        .iter()
        .map(|part| part_finding(part, &plan::read(root, delta, part)))
        .collect();
    object(json!({"findings": array_of(json!({"anyOf": findings}))}))
}

pub fn plan(known_ids: &[String]) -> Value {
    let by_position = json!({"type": "integer", "minimum": 1});
    let settle = match known_ids.is_empty() {
        true => by_position,
        false => json!({"anyOf": [{"enum": known_ids}, by_position]}),
    };
    object(json!({
        "claims": array_of(object(json!({"text": {"type": "string"}}))),
        "tasks": array_of(object(json!({
            "text": {"type": "string"},
            "settles": array_of(settle),
        }))),
    }))
}

pub fn review_diff(claims_text: &str) -> Value {
    let settles = ids_array(&live_claim_ids(claims_text));
    object(json!({
        "findings": array_of(object(json!({
            "text": {"type": "string"},
            "settles": settles,
        }))),
    }))
}

pub fn build(task: &Task) -> Value {
    let refs = match task.chore {
        true => json!({"type": "array", "items": {"type": "string"}, "maxItems": 0}),
        false => array_of(object(json!({
            "claim": {"enum": task.settles},
            "path": {"type": "string"},
            "symbol": {"type": "string"},
        }))),
    };
    object(json!({
        "outcome": {"enum": ["done", "blocked"]},
        "reason": {"type": "string"},
        "refs": refs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotate::annotate;

    const INDEX: &str = "---\ntitle: \"## Fake\"\n---\n\n## Intent\n\nBecause.\n\n### Detail\n\nMore.\n\n```\n## Hidden\n```\n";
    const DESIGN: &str = "Prose.\n\n## Approach\n\nDo it.\n";
    const CLAIMS: &str =
        "- [AC-1] First.\n  @ref a.rs#x\n- plain bullet\n  more\n- [intent] marked bullet\n- ~~[AC-2]~~ struck\n";
    const TASKS: &str = "- [ ] T1 one (settles AC-1)\n- [ ] T2 two (chore)\n\n## Follow-ups\n- [ ] T3 three (chore)\n";

    fn enumerated(part: &str, file: &str) -> Vec<String> {
        let schema = part_finding(part, file);
        schema["properties"]["anchor"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn every_enumerated_anchor_is_accepted_by_annotate() {
        let sample = [
            ("index", INDEX),
            ("design", DESIGN),
            ("claims", CLAIMS),
            ("tasks", TASKS),
        ];
        for (part, file) in sample {
            let anchors = enumerated(part, file);
            assert!(anchors.len() > 1, "{part}: {anchors:?}");
            for anchor in anchors {
                assert!(
                    annotate(part, file, &anchor, "q").is_some(),
                    "{part}: `{anchor}` is not accepted"
                );
            }
        }
    }

    #[test]
    fn the_anchors_are_the_ids_texts_and_headings_the_part_holds() {
        assert_eq!(
            enumerated("claims", CLAIMS),
            ["claims", "AC-1", "AC-2", "plain bullet", "marked bullet"]
        );
        assert_eq!(
            enumerated("tasks", TASKS),
            ["tasks", "T1", "T2", "T3", "Follow-ups"]
        );
        assert_eq!(enumerated("index", INDEX), ["index", "Intent", "Detail"]);
    }

    #[test]
    fn the_review_doc_schema_has_one_alternative_per_part() {
        let root = tempfile::tempdir().unwrap();
        let delta: Doc = serde_json::from_value(json!({
            "id": "DELTA-001", "path": "d/index.md", "title": "t", "type": "delta",
            "status": "draft", "body": "", "content_hash": "h",
        }))
        .unwrap();

        let schema = review_doc(root.path(), &delta);

        let alternatives = &schema["properties"]["findings"]["items"]["anyOf"];
        let parts: Vec<&str> = alternatives
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["properties"]["part"]["const"].as_str().unwrap())
            .collect();
        assert_eq!(parts, PARTS);
    }

    #[test]
    fn plan_settles_take_a_known_id_or_a_position() {
        let schema = plan(&["AC-1".to_string()]);

        let settle = &schema["properties"]["tasks"]["items"]["properties"]["settles"]["items"];
        assert_eq!(settle["anyOf"][0]["enum"], json!(["AC-1"]));
        assert_eq!(settle["anyOf"][1]["type"], "integer");
    }

    #[test]
    fn review_diff_settles_only_live_claims_or_nothing() {
        let live = review_diff(CLAIMS);
        let none = review_diff("");

        let items =
            |s: &Value| s["properties"]["findings"]["items"]["properties"]["settles"].clone();
        assert_eq!(items(&live)["items"]["enum"], json!(["AC-1"]));
        assert_eq!(items(&none)["maxItems"], 0);
    }

    #[test]
    fn build_refs_name_the_settled_claims_and_a_chore_has_none() {
        let tasks = tasks::parse(TASKS);

        let settling = build(&tasks.tasks[0]);
        let chore = build(&tasks.tasks[1]);

        assert_eq!(
            settling["properties"]["refs"]["items"]["properties"]["claim"]["enum"],
            json!(["AC-1"])
        );
        assert_eq!(chore["properties"]["refs"]["maxItems"], 0);
        assert_eq!(settling["required"], json!(["outcome", "reason", "refs"]));
    }
}
