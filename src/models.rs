use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

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

    fn rand_string(sz: usize) -> String {
        let mut rng = thread_rng();
        std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(sz)
            .collect()
    }

    pub fn create<P: AsRef<Path>>(ref_dir: P, playbook: P) -> std::io::Result<Job> {
        let tmp = std::path::Path::new("/tmp");
        let job_dir = tmp.join(format!("hotwings-{}", Job::rand_string(6)));
        debug!("Creating {:?}", job_dir);
        std::fs::create_dir(job_dir)?;
        let entry = std::fs::read_dir(job_dir)?;
        Ok(Job {
            timestamp: entry.metadata().unwrap().ctime(),
            dir: entry
        })
    }
}
