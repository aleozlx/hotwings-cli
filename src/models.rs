use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use colored::*;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub remotes: Option<Vec<Remote>>
}

pub const CONFIG: &str = ".hwclirc";
impl Config {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::write(path, toml::to_string(&self).unwrap())?;
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String,
    pub default: bool
}

impl Remote {
    pub fn default() -> Option<Remote> {
        let fname = dirs::home_dir().expect("Cannot determine the HOME directory.").join(CONFIG);
        if !fname.exists() { return None; }
        if let Ok(ref raw) = std::fs::read_to_string(&fname) {
            let mut config: Config = toml::from_str(raw).expect("Syntax error.");
            if let Some(ref mut remotes) = config.remotes {
                let selected: Vec<&Remote> = remotes.iter().filter(|remote| remote.default == true).collect();
                match selected.len() {
                    1 => { Some(selected[0].clone()) }
                    _ => { None }
                }
            }
            else { None }
        }
        else { None }
    }
}

impl std::fmt::Display for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} = {}", &self.name, &self.url)
    }
}

// pub enum RemoteRef<'a> {
//     Undefined,
//     Unique(Option<&'a Remote>),
//     Duplicate
// }

// impl Remote {
//     pub fn getter<'cfg>(config: &'cfg mut Config, name: &str) -> RemoteRef<'cfg> {
//         if let Some(ref remotes) = config.remotes {
//             let (selected, _): (Vec<&'cfg Remote>, Vec<&'cfg Remote>) = remotes.iter().partition(|remote| remote.name == name);
//             match selected.len() {
//                 1 => { RemoteRef::Unique(Some(selected[0])) }
//                 0 => { RemoteRef::Undefined }
//                 _ => { RemoteRef::Duplicate }
//             }
//         }
//         else { RemoteRef::Undefined }
//     }

//     pub fn setter<'cfg>(config: &'cfg mut Config, name: &str, url: &str) -> std::io::Result<RemoteRef<'cfg>> {
//         let fname = dirs::home_dir().expect("Cannot determine the HOME directory.").join(CONFIG);
//         let ret = if let Some(ref mut remotes) = config.remotes {
//             let (mut selected, _): (Vec<&'cfg mut Remote>, Vec<&'cfg mut Remote>) = remotes.iter_mut().partition(|remote| remote.name == name);
//             match selected.len() {
//                 1 => {
//                     selected[0].url = url.to_owned();
//                     Ok(RemoteRef::Unique(None))
//                 }
//                 0 => {
//                     remotes.push(Remote { name: name.to_owned(), url: url.to_owned() });
//                     let out = toml::to_string(&config).unwrap();
//                     Ok(RemoteRef::Undefined)
//                 }
//                 _ => { Ok(RemoteRef::Duplicate) }
//             }
//         }
//         else {
//             config.remotes = Some(vec![Remote { name: name.to_owned(), url: url.to_owned() }]);
//             let out = toml::to_string(&config).unwrap();
//             Ok(RemoteRef::Unique(None))
//         };
//         config.save(&fname)?;
//         return ret;
//     }
// }

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
        let mut p_tar = std::process::Command::new("sh")
            .args(&["-c", &format!("tar czf {} *", job_dir.join("a.tgz").to_str().unwrap())])
            .current_dir(ref_dir)
            .spawn()?;
        p_tar.wait()?;
        Ok(Job {
            timestamp: job_dir.metadata().unwrap().ctime(),
            dir: job_dir
        })
    }

    pub fn submit(&self, remote: &Remote) -> std::io::Result<()> {
        println!("Uploading {}/a.tgz", &self.dir.to_str().unwrap());
        let mut p_curl = std::process::Command::new("curl")
            .args(&[
                &remote.url, "--progress-bar", "--verbose",
                "-F", "job_archive=@a.tgz",
                "-F", &format!("playbook_name={}", &self.playbook()?.to_str().unwrap()),
                "-A", &format!("hwcli = {}", crate_version!())
            ])
            .current_dir(&self.dir)
            .spawn()?;
        p_curl.wait()?;
        Ok(())
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
