#[macro_use]
extern crate clap;

extern crate git2;
extern crate rustc_serialize;
extern crate toml;

use std::path::{Path, PathBuf};
use std::fs::{self, DirEntry};
use std::{io, env};
use clap::{App, ArgMatches, SubCommand};
use git2::build::RepoBuilder;

mod config;
use config::Config;

mod castle;
use castle::Castle;

struct World {
    root: PathBuf,
    home: PathBuf,
}

impl World {

    fn castles_path(&self) -> PathBuf {
        self.root.join("repos")
    }

    fn castles(&self) -> Result<Vec<Castle>, String> {
        let files = list_dirs(&self.castles_path()).map_err(|_| "Could not list castles")?;
        let (castles, fails): (Vec<Result<Castle,String>>, Vec<Result<Castle,String>>) = files
            .into_iter().map(|entry| Castle::new_for_path(entry.path()))
            .partition(|ref r| r.is_ok());

        for f in fails {
            println!("Failed to open castle: {}", f.err().unwrap());
        }

        Ok(castles.into_iter().map(|x| x.unwrap()).collect())
    }

    fn castle_for_name(&self, name: &str) -> Result<Castle, String> {
        Castle::new_for_path(self.castles_path().join(name))
    }

    fn resolve_target(&self, target: &str) -> Result<PathBuf, String> {
        Ok(self.home.as_path().join(target))
    }

    fn stat<P: AsRef<Path>>(&self, target: P) -> Result<castle::LinkType, String> {
        let metadata = target.as_ref().symlink_metadata().map_err(|_| "Could not stat file")?;
        let link = if metadata.is_dir() {
            castle::LinkType::Directory
        } else if metadata.file_type().is_symlink() {
            let target = target.as_ref().read_link().map_err(|_| "Could not read link")?;
            castle::LinkType::Symlink(target)
        } else {
            castle::LinkType::File
        };

        Ok(link)
    }
}

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

fn extract_name_from_url(url: &str) -> Option<&str> {
    let pos = url.rfind("/");
    if pos.is_none() {
        return None
    }
    let mut pos = pos.unwrap() + 1;
    let mut end = url.len();
    let name = &url[pos..];

    if name.ends_with(".git") {
        end = url.len() - 4;
    }

    if name.starts_with("dot-") {
        pos += 4;
    }

    Some(&url[pos..end])
}

const BOOTSTRAP_USAGE: &'static str = "
<repository> 'The repository to bootstrap from'
";

fn bootstrap(world: &World, matches: &ArgMatches) -> Result<(), git2::Error> {
    let repo_url = matches.value_of("repository").unwrap();
    let name = extract_name_from_url(&repo_url).ok_or(git2::Error::from_str("Invalid url"))?;
    let repo_path = world.castles_path().join(name);

    repo_clone(repo_url, repo_path.as_path())?;

    let cfg_path = repo_path.join("home.toml");
    if ! cfg_path.exists() {
        return Ok(());
    }

    let cfg = Config::open(cfg_path.as_path()).map_err(|e| git2::Error::from_str(&e))?;

    for (name, source) in &cfg.castles {
        let subdir = world.castles_path().join(name);
        println!("{}: \"{}\"", name, source.url);
        repo_clone(&source.url, &subdir)?;
    }

    Ok(())
}


fn list_dirs(dir: &Path) -> io::Result<Vec<DirEntry>> {
    let mut res: Vec<DirEntry> = Vec::new();

    if ! dir.is_dir() {
        return Ok(res)
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            res.push(entry);
        }
    }

    Ok(res)
}


const LINKS_USAGE: &'static str = "
<castle> 'The castle to show the links for'
";

fn show_links(world: &World, matches: &ArgMatches) -> Result<(), git2::Error> {
    let name = matches.value_of("castle").unwrap();
    let castle = world.castle_for_name(name).map_err(|e| git2::Error::from_str(&e))?;

    let files = castle.links().map_err(|e| git2::Error::from_str(&e))?;

    for f in files {
        println!("{} [{:?}] {}", f.path, f.kind, f.id);
    }

    Ok(())
}

const LIST_USAGE: &'static str = "
";

fn cmd_list(world: &World, matches: &ArgMatches) -> Result<(), git2::Error> {
    let home = world.castles_path();
    println!("listing...{:?}", home);

    let files = world.castles().map_err(|e| git2::Error::from_str(&e))?;

    for f in files {
        if let Some(name) = f.name() {
            println!("{:?}", name);
        }
    }

    Ok(())
}

const LINK_USAGE: &'static str = "
[castles]... 'The castles to link the files in'
";

fn cmd_link(world: &World, matches: &ArgMatches) -> Result<(), git2::Error> {
    let castles = if let Some(names) = matches.values_of("castles") {
        let mut castles: Vec<Castle> = Vec::new();
        for name in names {
            let c = world.castle_for_name(name).map_err(|e| git2::Error::from_str(&e))?;
            castles.push(c)
        }
        castles
    } else {
        world.castles().map_err(|e| git2::Error::from_str(&e))?
    };

    for castle in castles {
        println!("{:?}", castle.name())
    }

    Ok(())
}

const MAIN_USAGE: &'static str = "
-H, --home=[DIRECTORY] 'use this path instead of the home directory'
-R, --root=[DIRECTORY] 'root of our world, i.e. where all things are'
-v, --verbose          'show what is going on'
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
        .subcommand(SubCommand::with_name("links")
                    .args_from_usage(LINKS_USAGE))
        .subcommand(SubCommand::with_name("list")
                    .args_from_usage(LIST_USAGE))
        .subcommand(SubCommand::with_name("link")
                    .args_from_usage(LINK_USAGE))
        .get_matches();

    let home = matches
        .value_of("home")
        .map(PathBuf::from)
        .or_else(env::home_dir)
        .expect("could not determine home folder");

    let root = matches
        .value_of("root")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".homesick"));

    let world = World {
        home: home,
        root: root,
    };

    let res = match matches.subcommand() {
        ("bootstrap", Some(submatches)) => bootstrap(&world, submatches),
        ("links", Some(submatches)) => show_links(&world, submatches),
        ("list", Some(submatches)) => cmd_list(&world, submatches),
        ("link", Some(submatches)) => cmd_link(&world, submatches),
        ("", None)   => Err(git2::Error::from_str("Need command")),
        _            => unreachable!(),
    };

    if let Some(e) = res.err() {
        println!("E: {}", e);
    }
}
