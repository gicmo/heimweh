extern crate rustc_serialize;
extern crate git2;
extern crate toml;

use std::collections::BTreeMap;

use std::io::prelude::*;
use std::path::Path;
use std::fs::File;


#[derive(Debug, RustcDecodable)]
pub struct SourceEntry {
    pub url: String,
}

#[derive(Debug, RustcDecodable)]
pub struct Config {
    pub castles: BTreeMap<String, SourceEntry>,
}

impl Config {

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Config, String> {
        let mut file = File::open(&path).map_err(|_| "Could not open file")?;
        let mut data = String::new();

        file.read_to_string(&mut data)
            .map_err(|_| "Could not read file")?;

        let root: toml::Value = data.parse().map_err(|_| "Config file not valid TOML")?;
        let cfg: Config = toml::decode(root).ok_or_else(|| "Invalid config file")?;

        Ok(cfg)
    }
}

#[cfg(test)]
mod test {
    extern crate tempdir;

    use std::fs::File;
    use std::io::Write;
    use self::tempdir::TempDir;

    use super::*;

    #[test]
    fn parse() {
        let data = r#"
[castels]

  [castles.emacs]
  url = "https://github.com/gicmo/dot-emacs.git"

  [castles.files]
  url = "https://github.com/gicmo/dot-files.git"
"#;

        let tmp_dir = TempDir::new("heimweh").expect("create temp dir");

        let path = tmp_dir.path().join("home.toml");
        let mut fd = File::create(&path).expect("create temp file");
        fd.write_all(data.as_bytes()).expect("writing example config data");

        let cfg: Config = Config::open(&path).expect("Open config");
        println!("cfg: {:#?}", cfg);

        for (name, source) in &cfg.castles {
            println!("{}: \"{}\"", name, source.url);
        }
    }

}
