extern crate rustc_serialize;
extern crate git2;
extern crate toml;
#[macro_use]
extern crate clap;

use std::io::prelude::*;
use std::path::Path;
use std::fs::File;

use clap::{App, ArgMatches, SubCommand};

use git2::build::RepoBuilder;

fn repo_clone(remote: &str, local: &str) -> Result<git2::Repository, git2::Error> {
    let path = Path::new(local);
    let repo = RepoBuilder::new().clone(remote, path)?;

    for module in repo.submodules()?.iter_mut() {

        module.init(false)?;
        let url = module.url().unwrap();
        let rel_path = module.path();
        let sub_path = path.join(rel_path);

        println!("{} [{}]", module.name().unwrap(), path.to_str().unwrap());

        match RepoBuilder::new().clone(url, sub_path.as_path()) {
            Ok(_) => println!{"ok"},
            Err(e) => {
                println!("E: {}", e);
                return Err(e)
            },
        }

    }

    Ok(repo)
}


const BOOTSTRAP_USAGE: &'static str = "
<repository> 'The repository to bootstrap from'
<path> 'The local path'
";

fn bootstrap(matches: &ArgMatches) -> Result<(), git2::Error> {
    let repo_url = matches.value_of("repository").unwrap();
    let repo_path = matches.value_of("path").unwrap();

    repo_clone(repo_url, repo_path)?;

    Ok(())
}


#[derive(Debug, RustcDecodable)]
struct Config {
    castle: Vec<SourceEntry>,
}

#[derive(Debug, RustcDecodable)]
struct SourceEntry {
    url: String,
}

fn read_config(path: &str) -> Result<Config, String> {
    let mut file = File::open(&path).map_err(|_| "Could not open file")?;
    let mut data = String::new();

    file.read_to_string(&mut data)
        .map_err(|_| "Could not read file")?;

    let cfg: Config = toml::decode_str(&data).ok_or_else(|| "Invalid config file")?;
    Ok(cfg)
}

const MAIN_USAGE: &'static str = "
-v, --verbose 'show what is going on'
";

fn main() {

    let matches = App::new("heimweh")
        .about("heimweh - dot files roaming.")
        .version(crate_version!())
        .author(crate_authors!())
        .args_from_usage(MAIN_USAGE)
        .subcommand(SubCommand::with_name("bootstrap")
                    .about("Initialize everything")
                    .args_from_usage(BOOTSTRAP_USAGE))
        .get_matches();

    let res = match matches.subcommand() {
        ("bootstrap", Some(submatches)) => bootstrap(submatches),
        ("", None)   => Err(git2::Error::from_str("Need command")),
        _            => unreachable!(),
    };

    if let Some(e) = res.err() {
        println!("E: {}", e);
    }
}
