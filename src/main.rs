#[macro_use]
extern crate clap;

use std::os::unix::fs::MetadataExt;

mod cli;

fn main() {
    let matches = clap_app!(myapp =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand init => // storage: repo
            (about: "Create a new Hotwings job specification YAML (aka playbook)")
        )
        (@subcommand remote => // storage: $HOME
            (about: "Establish or switch server connection")
        )
        (@subcommand sub => // storage: /tmp/hotwings-task_id & chmod
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

    if let Some(matches) = matches.subcommand_matches("status") {
        // Print current dir
        let cwd = std::env::current_dir().expect("I/O Error: getting CWD");
        println!("Ref directory: {}", cwd.to_str().unwrap());

        let tmp = std::path::Path::new("/tmp");
        let my_uid = nix::unistd::Uid::current();
        let jobs: Vec<std::fs::DirEntry> = std::fs::read_dir(tmp).expect("I/O Error: enumerating /tmp").filter_map(Result::ok)
            .filter(|entry| entry.path().file_name().unwrap().to_str().unwrap().starts_with("hotwings-"))
            .filter(|entry| {
                if let Ok(metadata) = entry.metadata() { nix::unistd::Uid::from_raw(metadata.uid()) == my_uid }
                else { false }
            }).collect();
        for entry in jobs {
            println!("{:?}", entry);
        }

        

        // List YAML files
    }
}
