use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use colored::*;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

pub const SYM_REF_DIR: &str = ".ref";
pub const SYM_PLAYBOOK: &str = ".playbook";
pub struct Job {
    pub timestamp: i64,
    pub dir: PathBuf
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
                dir: entry.path()
            }).collect()
    }

    pub fn ref_dir(&self) -> std::io::Result<PathBuf> {
        let mut path = self.dir.clone();
        path.push(SYM_REF_DIR);
        std::fs::canonicalize(path)
    }

    pub fn playbook(&self) -> std::io::Result<PathBuf> {
        let mut path = self.dir.clone();
        path.push(SYM_PLAYBOOK);
        Ok(std::fs::canonicalize(path)?.as_path().strip_prefix(self.ref_dir()?).unwrap().to_path_buf())
    }

    fn rand_string(sz: usize) -> String {
        let mut rng = thread_rng();
        std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(sz)
            .collect()
    }

    pub fn create<P: AsRef<Path> + Clone>(ref_dir: P, playbook: P) -> std::io::Result<Job> {
        let tmp = Path::new("/tmp");
        let job_dir = tmp.join(format!("hotwings-{}", Job::rand_string(6)));
        debug!("Creating {:?}", job_dir);
        std::fs::create_dir(job_dir.clone())?;
        std::os::unix::fs::symlink(ref_dir.clone(), job_dir.join(SYM_REF_DIR))?;
        std::os::unix::fs::symlink(Path::new(".ref").join(playbook.as_ref().strip_prefix(ref_dir.clone()).unwrap()), job_dir.join(SYM_PLAYBOOK))?;
        let _tar = std::process::Command::new("sh")
            .args(&["-c", &format!("tar czf {} *", job_dir.join("a.tgz").to_str().unwrap())])
            .current_dir(ref_dir)
            .spawn()?;
        Ok(Job {
            timestamp: job_dir.metadata().unwrap().ctime(),
            dir: job_dir
        })
    }
}

impl std::fmt::Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}+{}", self.dir.to_str().unwrap(), match self.playbook() {
            Ok(playbook) => playbook.to_str().unwrap().blue(),
            Err(_) => "?".red()
        })
    }
}
