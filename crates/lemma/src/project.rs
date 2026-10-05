use crate::payload::Doc;
use anyhow::{Context, Result};
use lazyspec::engine::config::Config;
use lazyspec::engine::doc_json::doc_to_json;
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use std::path::{Path, PathBuf};

pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(".lazyspec.toml").is_file())
        .map(Path::to_path_buf)
}

pub fn load_store(root: &Path) -> Result<Store> {
    let config = Config::load(root, &RealFileSystem).context("loading .lazyspec.toml")?;
    Store::load(root, &config).context("loading lazyspec store")
}

pub fn specs_from_store(root: &Path, names: &[&str]) -> Vec<Doc> {
    if names.is_empty() {
        return Vec::new();
    }
    let Ok(store) = load_store(root) else {
        return Vec::new();
    };
    names
        .iter()
        .filter_map(|name| store.resolve_shorthand(name).ok())
        .filter_map(|meta| {
            let mut json = doc_to_json(meta);
            json["body"] = store.get_body_raw(&meta.path, &RealFileSystem).ok()?.into();
            json["parts"] = meta
                .parts
                .iter()
                .map(|part| {
                    let body = store.get_part_body_raw(&part.path, &RealFileSystem).ok()?;
                    Some(serde_json::json!({
                        "name": part.name,
                        "path": part.path.to_string_lossy(),
                        "body": body,
                    }))
                })
                .collect::<Option<Vec<_>>>()?
                .into();
            serde_json::from_value::<Doc>(json).ok().map(|doc| Doc {
                from_store: true,
                ..doc
            })
        })
        .filter(|doc| doc.doc_type == "spec")
        .collect()
}
