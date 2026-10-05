use crate::payload::Doc;
use anyhow::{Context, Result};
use lazyspec::engine::config::Config;
use lazyspec::engine::doc_json::doc_to_json;
use lazyspec::engine::document::DocMeta;
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

fn stored_doc(store: &Store, meta: &DocMeta) -> Option<Doc> {
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
}

pub fn docs_from_store(store: &Store) -> Vec<Doc> {
    let mut docs: Vec<Doc> = store
        .all_docs()
        .into_iter()
        .filter(|meta| matches!(meta.doc_type.to_string().as_str(), "spec" | "delta"))
        .filter_map(|meta| stored_doc(store, meta))
        .collect();
    docs.sort_by(|a, b| a.id.cmp(&b.id));
    docs
}
