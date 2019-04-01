use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub struct Job {
    pub timestamp: i64,
    pub dir: std::fs::DirEntry
}

impl Job {
    pub fn list() -> Vec<Job> {
        let my_uid = nix::unistd::Uid::current();
        let tmp = std::path::Path::new("/tmp");
        std::fs::read_dir(tmp).expect("I/O Error: enumerating /tmp").filter_map(Result::ok)
            .filter(|entry| entry.path().file_name().unwrap().to_str().unwrap().starts_with("hotwings-"))
            .filter(|entry| {
                if let Ok(metadata) = entry.metadata() { nix::unistd::Uid::from_raw(metadata.uid()) == my_uid }
                else { false }
            })
            .map(|entry| Job {
                timestamp: entry.metadata().unwrap().ctime(),
                dir: entry
            }).collect()
    }

    pub fn ref_dir(&self) -> std::io::Result<PathBuf> {
        let mut path = self.dir.path();
        path.push(".ref");
        std::fs::canonicalize(path)
    }

    pub fn playbook(&self) -> std::io::Result<PathBuf> {
        let mut path = self.dir.path();
        path.push(".playbook");
        Ok(std::fs::canonicalize(path)?.as_path().strip_prefix(self.ref_dir()?).unwrap().to_path_buf())
    }
}
