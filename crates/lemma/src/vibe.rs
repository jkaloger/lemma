use crate::build;
use std::path::Path;

pub fn run(root: &Path) -> Result<(), String> {
    build::require_clean(root)
}
