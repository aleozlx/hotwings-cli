#[macro_use]
extern crate clap;
extern crate colored;
#[macro_use]
extern crate log;
extern crate dirs;
extern crate fern;

mod cli;
mod models;

use std::path::{Path, PathBuf};
use colored::*;
use models::Job;

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
    for job in jobs.iter().take(10) {
        println!("{:?}", job.dir);
    }    
}

fn status<'a>(matches: &clap::ArgMatches<'a>) {
    // Print current dir
    let cwd = std::fs::canonicalize(std::env::current_dir().expect("I/O Error: getting CWD")).unwrap();
    println!("Ref directory: {}", cwd.to_str().unwrap().blue());
    let mut jobs: Vec<Job> = Job::list().into_iter()
        .filter(|job| {
            if let Ok(ref_dir) = job.ref_dir() {
                debug!("{:?} => {:?}", job.dir, ref_dir);
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

    // List YAML files
    
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
        )
        (@subcommand sub => // storage: /tmp/hotwings-task_id & chmod
            (@arg PREPARE: --prepare "Prepare a submission tarball but do not submit.")
            (@arg PLAYBOOK: +required "YAML playbook")
            (about: "Check & submit a playbook to a Hotwings job system")
        )
        (@subcommand list => // storage: /tmp/hotwings-* owned by current user
            (about: "List jobs submitted")
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
}
