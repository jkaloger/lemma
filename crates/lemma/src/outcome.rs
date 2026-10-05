use crate::payload::Finding;

pub struct Outcome {
    pub findings: Vec<Finding>,
    pub refused: bool,
}

impl Outcome {
    pub fn refuse(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            refused: true,
        }
    }

    pub fn done(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            refused: false,
        }
    }
}
