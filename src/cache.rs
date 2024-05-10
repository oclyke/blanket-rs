use blake3::Hash as Blake3Hash;
use blake3::Hasher as Blake3Hasher;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// Cache
/// Trait for caching metadata about targets
pub trait Cache<H>
where
    H: Eq,
{
    fn set(&self, key: &Path, hash: &H, deps: &HashMap<PathBuf, H>);
    fn get(&self, key: &Path) -> Option<(H, HashMap<PathBuf, H>)>;
    fn hash(&mut self, filepath: &Path) -> H;
}

/// NoCache
/// The No Cache Cache
/// This cache does not cache anything
pub struct NoCache {}

impl NoCache {
    pub fn new() -> Self {
        Self {}
    }
}

impl Cache<u64> for NoCache {
    fn set(&self, _key: &Path, _hash: &u64, _deps: &HashMap<PathBuf, u64>) {}

    fn get(&self, _key: &Path) -> Option<(u64, HashMap<PathBuf, u64>)> {
        None
    }

    fn hash(&mut self, _filepath: &Path) -> u64 {
        0
    }
}

const HASH_EXT: &'static str = "blake3";

/// FsCache
pub struct FsCache {
    path: PathBuf,
}

impl FsCache {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn initialize(&self) {
        std::fs::create_dir_all(&self.path).expect("Failed to create cache directory");
    }

    fn target_location(&self, key: &Path) -> PathBuf {
        append_ext_hash(&self.path.join(key).join("target"))
    }

    fn dep_dir(&self, key: &Path) -> PathBuf {
        self.path.join(key).join("inputs")
    }

    fn dep_location(&self, key: &Path, dep: &Path) -> PathBuf {
        append_ext_hash(&self.dep_dir(key).join(dep))
    }

    fn get_dep_from_location(&self, key: &Path, location: &Path) -> PathBuf {
        location
            .strip_prefix(&self.dep_dir(key))
            .unwrap()
            .to_path_buf()
    }
}

impl Cache<Blake3Hash> for FsCache {
    fn set(&self, key: &Path, hash: &Blake3Hash, deps: &HashMap<PathBuf, Blake3Hash>) {
        // write the hash of the target
        let path = self.target_location(key);
        std::fs::create_dir_all(&path.parent().unwrap()).expect("Failed to create directory");
        std::fs::write(&path, hash.to_string()).expect("Failed to write hash");

        // write the hashes of the dependencies
        std::fs::create_dir_all(&self.dep_dir(key)).expect("Failed to create directory");
        for (dep, dep_hash) in deps {
            let path = self.dep_location(key, dep);
            std::fs::create_dir_all(&path.parent().unwrap()).expect("Failed to create directory");
            std::fs::write(&path, dep_hash.to_string()).expect("Failed to write hash");
        }
    }

    fn get(&self, key: &Path) -> Option<(Blake3Hash, HashMap<PathBuf, Blake3Hash>)> {
        let path = self.target_location(key);
        let hash = std::fs::read_to_string(path).ok()?;
        let hash = hash.parse().ok()?;

        let mut deps = HashMap::new();
        let dep_dir = self.dep_dir(key);

        // find all the files that end in the .hash extension
        // using walkdir
        let dependencies = walkdir::WalkDir::new(&dep_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == HASH_EXT)
                    .unwrap_or(false)
            })
            .map(|entry| entry.path().to_path_buf())
            .collect::<Vec<PathBuf>>();

        // now read all the hashes for the corresponding dependencies
        for dep in dependencies {
            let dep_hash = std::fs::read_to_string(&dep).ok()?;
            let dep_hash = dep_hash.parse().ok()?;
            let dep_with_ext = self.get_dep_from_location(key, &dep);
            let dep = dep_with_ext.with_extension("");
            deps.insert(dep, dep_hash);
        }

        Some((hash, deps))
    }

    fn hash(&mut self, filepath: &Path) -> Blake3Hash {
        let bytes = std::fs::read(filepath).expect("Failed to read file");
        blake3::hash(&bytes)
    }
}

/// Appends the extension ".hash" to the given path
/// If the path already has an extension the new extension will be appended after the existing one
/// If the path is a directory the extension will be appended to the directory name
/// If the path is empty
fn append_ext_hash(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    let new_ext = match path.extension() {
        Some(ext) => format!("{}.{}", ext.to_string_lossy(), HASH_EXT),
        None => format!("{}", HASH_EXT),
    };
    path.set_extension(new_ext);
    path
}
