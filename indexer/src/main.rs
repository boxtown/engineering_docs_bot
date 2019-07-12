use eddy::{analysis, storage};
use failure::Error;
use lambda_runtime::error::HandlerError;
use lambda_runtime::{lambda, Context};
use serde_json::Value;
use simple_logger;
use std::{env, fs};
use walkdir::{DirEntry, WalkDir};

fn clone_engineering_docs(
    github_token: &str,
    repo_rel_url: &str,
    repo_path: &str,
) -> Result<(), Error> {
    let url = format!("https://{}@github.com/{}", github_token, repo_rel_url);
    git2::Repository::clone(&url, repo_path)
        .map(|_| ())
        .map_err(|e| Error::from_boxed_compat(Box::new(e)))
}

fn index_engineering_docs(repo_path: &str, conn_str: &str) -> Result<(), Error> {
    let mut keyword_map = analysis::KeywordMap::new();

    for entry in WalkDir::new(repo_path) {
        let entry = entry?;
        if should_not_index(&entry) {
            continue;
        }

        let file = fs::read_to_string(entry.path())?;
        let keywords = analysis::analyze_keywords(file)?;

        let doc_key = entry
            .path()
            .to_string_lossy()
            .trim_start_matches(repo_path)
            .to_owned();
        keyword_map.insert(doc_key, keywords);
    }

    let reverse_keyword_map = analysis::reverse_keyword_map(&keyword_map);
    storage::redis::save_reverse_keyword_map(conn_str, reverse_keyword_map)?;

    Ok(())
}

fn should_not_index(entry: &DirEntry) -> bool {
    entry.file_type().is_dir() || !entry.file_name().to_string_lossy().ends_with(".md")
}

fn handler(event: Value, _: Context) -> Result<Value, HandlerError> {
    let github_token = env::var("github_token")?;
    let repo_rel_url = env::var("repo_rel_url")?;
    let repo_path = env::var("repo_path")?;
    let conn_str = env::var("redis_url")?;
    clone_engineering_docs(&github_token, &repo_rel_url, &repo_path)?;
    index_engineering_docs(&repo_path, &conn_str)?;
    Ok(event)
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    lambda!(handler);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_index_engineering_docs() {
        let repo_path = "FIXME";
        let conn_str = "redis://127.0.0.1";
        index_engineering_docs(&repo_path, &conn_str).unwrap();
    }
}
