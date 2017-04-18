
use std::io::{self};
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

extern crate git2;


pub struct Castle {
    repo: git2::Repository,
}

impl Castle {
    pub fn new_for_path<P: AsRef<Path>>(path: P) -> Result<Castle, String> {
        let repo = git2::Repository::open(path).map_err(|e| format!("could not open castle: {}", e))?;
        Ok(Castle{repo: repo})
    }

    pub fn name(&self) -> Option<&OsStr> {
        self.repo.workdir().and_then(|p| p.file_name())
    }

    pub fn links(&self) -> Result<Vec<Link>, String> {
        let root = self.tree_for_head().map_err(|e| format!("git error: {}", e))?;

        let bobj = root.get_name("home")
            .ok_or("no 'home' dir found in castle")?
        .to_object(&self.repo).expect("tree object");

        let bridge = bobj.as_tree().expect("A tree");

        list_file_in_tree(&self.repo, bridge, None).map_err(|e| format!("git error: {}", e))
    }

    pub fn resolve_link(&self, link: &Link) -> io::Result<PathBuf> {
        self.resolve_path(&link.target())
    }

    fn tree_for_head(&self) -> Result<git2::Tree, git2::Error> {
        let head = self.repo.head()?.resolve()?.target().unwrap();
        let commit = self.repo.find_commit(head)?;
        commit.tree()
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        let wdir = self.repo.workdir().expect("Could not obtain workdir for castle");
        wdir.join("home").join(path).canonicalize()
    }

    pub fn path_is_inside<P: AsRef<Path>>(&self, path: P) -> bool {
        let wdir = self.repo.workdir().expect("Could not obtain workdir for castle");
        return path.as_ref().starts_with(wdir)
    }

}

#[derive(Debug)]
pub enum LinkType {
    Directory,
    File,
    Symlink(PathBuf),
}

#[derive(Debug)]
pub struct Link {
    pub path: String,
    pub id: git2::Oid,
    pub kind: LinkType,
}

impl Link {

    pub fn target(&self) -> PathBuf {
        match &self.kind {
            &LinkType::File | &LinkType::Directory => {
                PathBuf::from(&self.path)
            },
            &LinkType::Symlink(ref l) => {
                let p = PathBuf::from(&self.path);
                let p = p.parent().unwrap_or(Path::new("."));
                p.join(l)
            }
        }
    }

    pub fn is_dir(&self) -> bool {
        match &self.kind {
            &LinkType::Directory => true,
            _ => false
        }
    }
}

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

fn osstr_from_bytes(slice: &[u8]) -> &OsStr {
    OsStrExt::from_bytes(slice)
}


fn list_file_in_tree(repo: &git2::Repository, root: &git2::Tree, path: Option<&Path>)
                     -> Result<Vec<Link>, git2::Error> {

    let mut res: Vec<Link> = Vec::new();

    for entry in root.iter() {
        let id = entry.id();
        let kind = entry.kind();
        let name = entry.name().expect("TreeEntry needs a name");
        let filepath = path.map(|p| p.join(name)).unwrap_or(PathBuf::from(name));
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

            Some(git2::ObjectType::Blob) => {
                let kind = if entry.filemode() == 0o120000 {
                    let obj = entry.to_object(repo).expect("blob object");
                    let blob = obj.as_blob().expect("A blob");
                    let content = blob.content();
                    let content = PathBuf::from(osstr_from_bytes(content));
                    LinkType::Symlink(content)
                } else {
                    LinkType::File
                };

                res.push(Link {
                    path: pathstr,
                    id: id,
                    kind: kind,
                });
            }
            _ => {
                println!("Unexpected kind in Tree: {:?}", kind);
            }
        }
    }

    Ok(res)
}
