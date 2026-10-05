use crate::payload::{Finding, Written};

pub struct Outcome {
    pub findings: Vec<Finding>,
    pub wrote: Vec<Written>,
    pub refused: bool,
}

impl Outcome {
    pub fn refuse(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            wrote: Vec::new(),
            refused: true,
        }
    }

    pub fn done(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            wrote: Vec::new(),
            refused: false,
        }
    }

    pub fn wrote(mut self, wrote: Vec<Written>) -> Self {
        self.wrote = wrote;
        self
    }
}
