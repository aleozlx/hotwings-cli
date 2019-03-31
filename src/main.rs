#[macro_use]
extern crate clap;

fn main() {
    let matches = clap_app!(myapp =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand init =>
            (about: "Creates a new hotwings job specification YAML (aka playbook)")
        )
    ).get_matches();
}
