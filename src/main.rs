#[macro_use]
extern crate clap;
extern crate colored;
#[macro_use]
extern crate log;
extern crate dirs;
extern crate fern;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod cli;
mod models;

use std::path::{Path, PathBuf};
use colored::*;
use models::{Job, Remote};

fn setup_logger(verbose: u64) -> Result<(), fern::InitError> {
    // let ref log_dir = dirs::home_dir().expect("Cannot determine the HOME directory.").join(".hwcli");
    // if !Path::new(log_dir).exists() { std::fs::create_dir(log_dir)?; }
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.level(),
                message
            ))
        })
        .level(match verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            3 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Trace
        })
        // .chain(fern::log_file(log_dir.join("hotwings.log"))?)
        .chain(std::io::stderr())
        .apply()?;
    Ok(())
}

fn list<'a>(matches: &clap::ArgMatches<'a>) {
    let mut jobs: Vec<Job> = Job::list();
    jobs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    if jobs.len() == 0 {
        println!("No job is found associated to this directory.");
    }
    let top_n = value_t!(matches.value_of("TOPN"), usize).unwrap_or(10);
    for job in jobs.iter().take(top_n) {
        println!("{}", job);
    }    
}

fn sub<'a>(matches: &clap::ArgMatches<'a>) {
    let cwd = std::fs::canonicalize(std::env::current_dir().expect("I/O Error: getting CWD")).unwrap();
    println!("Ref directory: {}", cwd.to_str().unwrap().blue());
    let playbook_name = matches.value_of("PLAYBOOK").unwrap();
    match std::fs::canonicalize(playbook_name) {
        Ok(playbook) => {
            match Job::create(cwd, playbook) {
                Ok(job) => {
                    if matches.is_present("PREPARE") {
                        println!("The job has been prepared but not submitted per user's request.");
                        return;
                    }
                    // job.submit();
                }
                Err(e) => {
                    error!("I/O Error: creating a job");
                    error!("{}", e);
                }
            }
        }
        Err(e) => {
            error!("I/O Error: reading {}", playbook_name);
            error!("{}", e);
        }
    }
}

fn status<'a>(matches: &clap::ArgMatches<'a>) {
    // Print current dir
    let cwd = std::fs::canonicalize(std::env::current_dir().expect("I/O Error: getting CWD")).unwrap();
    println!("Ref directory: {}", cwd.to_str().unwrap().blue());
    let mut jobs: Vec<Job> = Job::list().into_iter()
        .filter(|job| {
            if let Ok(ref_dir) = job.ref_dir() {
                debug!("{} @ {}", job, ref_dir.to_str().unwrap().blue());
                cwd == ref_dir
            }
            else { false }
        }).collect();
    if jobs.len() > 0 {
        jobs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if let Ok(playbook) = jobs[0].playbook() {
            println!("Last playbook submitted: {}", playbook.to_str().unwrap().blue());
        }
    }
    println!("{} job(s) have been submitted from this directory.", jobs.len());  
}

fn remote<'a>(matches: &clap::ArgMatches<'a>) {
    const CONFIG: &str = ".hwclirc";
    let fname = dirs::home_dir().expect("Cannot determine the HOME directory.").join(CONFIG);
    debug!("Config file: {}", fname.to_str().unwrap());
    if !fname.exists() {
        debug!("Creating a new config file");
        let _touch = std::process::Command::new("touch")
            .args(&[fname.to_str().unwrap()])
            .spawn().expect("I/O Error");
    }
    let remote_name = matches.value_of("NAME").unwrap();
    if let Ok(ref raw) = std::fs::read_to_string(fname) {
        let mut config: models::Config = toml::from_str(raw).expect("Syntax error.");
        if let Some(ref mut remotes) = config.remotes {
            let (mut selected, _): (Vec<&mut Remote>, Vec<&mut Remote>) = remotes.iter_mut().partition(|remote| remote.name == remote_name);
            match selected.len() {
                1 => {
                    if let Some(url) = matches.value_of("URL") {
                        selected[0].url = url.to_owned();
                        let out = toml::to_string(&config).unwrap();
                        println!("{}", out);
                        // TODO save file
                    }
                    else {
                        println!("url = {}", selected[0].url);
                    }
                }
                0 => {
                    if let Some(url) = matches.value_of("URL") {
                        remotes.push(Remote { name: remote_name.to_owned(), url: url.to_owned() });
                        let out = toml::to_string(&config).unwrap();
                        println!("{}", out);
                        // TODO save file
                    }
                    else {
                        println!("A remoted named \"{}\" is undefined.", remote_name);
                    }
                }
                _ => { println!("There are duplicate remotes named  \"{}\"", remote_name); }
            }
        }
        else {
            if let Some(url) = matches.value_of("URL") {
                config.remotes = Some(vec![Remote { name: remote_name.to_owned(), url: url.to_owned() }]);
                let out = toml::to_string(&config).unwrap();
                println!("{}", out);
                // TODO save file
            }
            else {
                // There is no entry, so it must be undefined.
                println!("A remoted named \"{}\" is undefined.", remote_name);
            }
            
        }

    }
}

macro_rules! subcommand {
    ($subcmd:ident, $matches:ident) => {
        if let Some(matches) = $matches.subcommand_matches(stringify!($subcmd)) {
            ($subcmd)(matches)
        }
    }
}

fn main() {
    let args = clap_app!(hwcli =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg VERBOSE: --verbose -v ... "Logging verbosity")
        (@subcommand new => // storage: repo
            (about: "Create a new Hotwings job specification YAML (aka playbook)")
        )
        (@subcommand remote => // storage: $HOME
            (about: "Establish or switch server connection")
            (@arg NAME: +required "Remote name")
            (@arg URL: "If present, overwrite the remote URL, otherwise print.")
        )
        (@subcommand sub => // storage: /tmp/hotwings-task_id & chmod
            (@arg PREPARE: --prepare "Prepare a submission tarball but do not submit.")
            (@arg PLAYBOOK: +required "YAML playbook")
            (about: "Check & submit a playbook to a Hotwings job system")
        )
        (@subcommand list => // storage: /tmp/hotwings-* owned by current user
            (about: "List jobs submitted")
            (@arg TOPN: "Only list the last N jobs.")
        )
        (@subcommand status =>
            (about: "Check status on any job submission based on the current directory")
        )
        (@subcommand logs =>
            (about: "Print logs to stdout and stderr")
        )
    ).get_matches();
    setup_logger(args.occurrences_of("VERBOSE")).expect("Logger Error.");
    subcommand!(status, args);
    subcommand!(list, args);
    subcommand!(sub, args);
    subcommand!(remote, args);
}
