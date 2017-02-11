extern crate rustc_serialize;
extern crate git2;
extern crate toml;

#[macro_use]
extern crate clap;

use std::path::Path;
use clap::{App, ArgMatches, SubCommand};
use git2::build::RepoBuilder;

mod config;
use config::Config;

fn repo_clone(remote: &str, path: &Path) -> Result<git2::Repository, git2::Error> {
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
    let root_path = Path::new(matches.value_of("path").unwrap());
    let repo_path = root_path.join("root");

    repo_clone(repo_url, repo_path.as_path())?;

    let cfg_path = repo_path.join("home.toml");
    if ! cfg_path.exists() {
        return Ok(());
    }

    let cfg = Config::open(cfg_path.as_path()).map_err(|e| git2::Error::from_str(&e))?;

    for (name, source) in &cfg.castles {
        let subdir = root_path.join(name);
        println!("{}: \"{}\"", name, source.url);
        repo_clone(&source.url, &subdir)?;
    }

    Ok(())
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
