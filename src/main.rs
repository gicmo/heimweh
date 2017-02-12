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


fn list_files(dir: &Path) -> io::Result<Vec<DirEntry>> {
    let mut res: Vec<DirEntry> = Vec::new();

    if ! dir.is_dir() {
        return Ok(res)
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        res.push(entry);

        if path.is_dir() {
            let mut children = list_files(&path)?;
            res.append(& mut children);
        }
    }

    Ok(res)
}


const LINKS_USAGE: &'static str = "
<castle> 'The castle to show the links for'
";

#[derive(Debug)]
enum LinkType {
    Directory,
    File,
    Symlink(String),
}
struct Link {
    path: String,
    id: git2::Oid,
    kind: LinkType,
}

fn list_file_in_tree(repo: &git2::Repository, root: &git2::Tree, path: Option<&Path>)
                     -> Result<Vec<Link>, git2::Error> {
    let mut res: Vec<Link> = Vec::new();

    for entry in root.iter() {
        let id = entry.id();
        let kind = entry.kind();
        let name = entry.name().expect("TreeEntry needs a name");
        let filepath = path.unwrap_or(Path::new("/")).join(name);
        let pathstr = filepath.to_str().expect("A string").to_string();

        match kind {
            Some(git2::ObjectType::Tree) => {
                let obj = entry.to_object(repo).expect("tree object");
                let subtree = obj.as_tree().expect("A tree");
                res.push(Link {
                    path: pathstr,
                    id: id,
                    kind: LinkType::Directory,
                });

                let mut children = list_file_in_tree(repo, subtree, Some(filepath.as_path()))?;
                res.append(& mut children);
            }
            //TODO: Detect symlinks
            Some(git2::ObjectType::Blob) => {
                res.push(Link {
                    path: pathstr,
                    id: id,
                    kind: LinkType::File,
                });
            }
            _ => {
                println!("Unexpected kind in Tree: {:?}", kind);
            }
        }
    }

    Ok(res)
}

fn show_links(matches: &ArgMatches) -> Result<(), git2::Error> {
    let name = matches.value_of("castle").unwrap();
    let mut home = matches
        .value_of("home")
        .map(PathBuf::from)
        .or_else(env::home_dir)
        .unwrap();

    home.push(".homesick");
    home.push("repos");
    home.push(name);

    let repo = git2::Repository::open(home)?;
    let head = repo.head()?.resolve()?.target().unwrap();
    let commit = repo.find_commit(head)?;
    let root = commit.tree()?;

    let bobj = root.get_name("home")
        .ok_or(git2::Error::from_str("no 'home' dir found in castle"))?
        .to_object(&repo).expect("tree object");

    let bridge = bobj.as_tree().expect("A tree");

    let files = list_file_in_tree(&repo, &bridge, None)?;

    for f in files {
        println!("{} [{:?}] {}", f.path, f.kind, f.id);
    }

    Ok(())
}

const MAIN_USAGE: &'static str = "
-H, --home    'use this path instead of the home directory'
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
        .subcommand(SubCommand::with_name("links")
                    .args_from_usage(LINKS_USAGE))
        .get_matches();

    let res = match matches.subcommand() {
        ("bootstrap", Some(submatches)) => bootstrap(submatches),
        ("links", Some(submatches)) => show_links(submatches),
        ("", None)   => Err(git2::Error::from_str("Need command")),
        _            => unreachable!(),
    };

    if let Some(e) = res.err() {
        println!("E: {}", e);
    }
}
