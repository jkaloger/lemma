use crate::claims::{heading_level, live_lines};
use regex::Regex;
use std::sync::OnceLock;

pub const FOLLOW_UPS: &str = "Follow-ups";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub number: u32,
    pub line: usize,
    pub ticked: bool,
    pub text: String,
    pub settles: Vec<String>,
    pub chore: bool,
    pub follow_up: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct Tasks {
    pub tasks: Vec<Task>,
    pub malformed: Vec<Malformed>,
}

impl Tasks {
    pub fn main(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter().filter(|t| !t.follow_up)
    }

    pub fn first_unticked(&self) -> Option<&Task> {
        let unticked = |t: &&Task| !t.ticked;
        self.main()
            .find(unticked)
            .or_else(|| self.tasks.iter().filter(|t| t.follow_up).find(unticked))
    }

    pub fn named(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }
}

fn checkbox() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^- \[( |x|X)\]\s*(.*)$").unwrap())
}

fn task_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^T([0-9]+)\s+(.*?)\s*\((?:settles\s+([A-Za-z0-9\-,\s]+)|(chore))\)\s*$")
            .unwrap()
    })
}

fn task_of(line: usize, ticked: bool, rest: &str, follow_up: bool) -> Option<Task> {
    let caps = task_pattern().captures(rest)?;
    let settles: Vec<String> = caps
        .get(3)
        .map(|m| {
            m.as_str()
                .split(',')
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let chore = caps.get(4).is_some();
    if !chore && settles.is_empty() {
        return None;
    }
    Some(Task {
        id: format!("T{}", &caps[1]),
        number: caps[1].parse().ok()?,
        line,
        ticked,
        text: caps[2].to_string(),
        settles,
        chore,
        follow_up,
    })
}

pub fn parse(body: &str) -> Tasks {
    let mut parsed = Tasks::default();
    let mut follow_up = false;
    for (line, text) in live_lines(body) {
        if let Some((_, name)) = heading_level(text) {
            follow_up = name.eq_ignore_ascii_case(FOLLOW_UPS);
            continue;
        }
        let Some(caps) = checkbox().captures(text) else {
            continue;
        };
        let ticked = &caps[1] != " ";
        match task_of(line, ticked, &caps[2], follow_up) {
            Some(task) => parsed.tasks.push(task),
            None => parsed.malformed.push(Malformed {
                line,
                text: text.to_string(),
            }),
        }
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASKS: &str = "\
- [x] T1 add `Balance` type (settles AC-1)
- [ ] T2 balance endpoint (settles AC-2, AC-3)
- [ ] T3 wire route into the router (chore)

## Follow-ups
- [ ] T4 page the read (settles AC-2)
";

    #[test]
    fn parses_main_tasks_and_follow_ups() {
        let parsed = parse(TASKS);
        assert!(parsed.malformed.is_empty());
        let ids: Vec<_> = parsed
            .tasks
            .iter()
            .map(|t| (t.id.as_str(), t.ticked, t.follow_up))
            .collect();
        assert_eq!(
            ids,
            vec![
                ("T1", true, false),
                ("T2", false, false),
                ("T3", false, false),
                ("T4", false, true)
            ]
        );
        assert_eq!(parsed.tasks[1].settles, vec!["AC-2", "AC-3"]);
        assert_eq!(parsed.tasks[1].text, "balance endpoint");
        assert!(parsed.tasks[2].chore && parsed.tasks[2].settles.is_empty());
        assert_eq!(parsed.tasks[3].line, 6);
    }

    #[test]
    fn tasks_after_the_follow_ups_heading_are_follow_ups() {
        let parsed = parse("- [x] T1 a (chore)\n\n## Follow-ups\n- [ ] T2 b (chore)\n");
        assert!(!parsed.tasks[0].follow_up && parsed.tasks[1].follow_up);
        assert_eq!(parsed.main().count(), 1);
    }

    #[test]
    fn a_task_without_settles_or_chore_is_malformed() {
        let parsed = parse("- [ ] T1 just do it\n- [ ] do it (chore)\n- [ ] T2 x (settles )\n");
        assert!(parsed.tasks.is_empty());
        assert_eq!(
            parsed.malformed.iter().map(|m| m.line).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn commented_and_fenced_tasks_are_ignored() {
        let parsed = parse("<!-- - [ ] T1 example (chore) -->\n```\n- [ ] nope\n```\n");
        assert!(parsed.tasks.is_empty() && parsed.malformed.is_empty());
    }
}
