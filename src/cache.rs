use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Cache<T>
where
    T: Serialize + DeserializeOwned,
{
    directory: PathBuf,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Cache<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new(tag: &str) -> Self {
        let directory = Path::new(".cache").join(tag);
        Self {
            directory,
            _phantom: std::marker::PhantomData,
        }
    }

    fn cache_file_path(&self, id: &str) -> PathBuf {
        let mut path = self.directory.clone();
        path.push(id);
        path.with_extension("json")
    }

    pub fn get(&self, id: &str) -> Result<Option<T>> {
        let path = self.cache_file_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read cache file {:?}", path))?;

        serde_json::from_str(&contents)
            .map(Some)
            .with_context(|| format!("Failed to deserialize cache file {:?}", path))
    }

    pub fn set(&self, id: &str, value: &T) -> Result<()> {
        let path = self.cache_file_path(id);
        fs::create_dir_all(&self.directory)
            .with_context(|| format!("Failed to create cache directory {:?}", self.directory))?;

        let contents =
            serde_json::to_string_pretty(value).context("Failed to serialize cache value")?;

        fs::write(&path, contents).with_context(|| format!("Failed to write cache file {:?}", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Person {
        name: String,
        age: i32,
    }

    #[test]
    fn simple_roundtrip() -> Result<()> {
        let cache = Cache::<Person>::new("test");

        let alice = Person {
            name: "Alice".to_string(),
            age: 30,
        };

        cache.set("alice", &alice)?;
        assert_eq!(cache.get("alice")?, Some(alice));

        let updated_alice = Person {
            name: "Alice Smith".to_string(),
            age: 31,
        };

        cache.set("alice", &updated_alice)?;
        assert_eq!(cache.get("alice")?, Some(updated_alice));

        Ok(())
    }
}
