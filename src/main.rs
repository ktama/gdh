#![allow(unused)]

use anyhow::{Context, Result};
use clap::Parser;
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use std::cell::BorrowError;
use std::fs::{self, DirEntry};
use std::io::{self, Write};
use std::path::{self, Path};

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = ".")]
    path: std::path::PathBuf,
}

struct VisitDir {
    root: Box<dyn Iterator<Item = io::Result<DirEntry>>>,
    children: Box<dyn Iterator<Item = VisitDir>>,
}

impl VisitDir {
    fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let root = Box::new(fs::read_dir(&path)?);
        let children = Box::new(fs::read_dir(&path)?.filter_map(|e| {
            let e = e.ok()?;
            if e.file_type().ok()?.is_dir() {
                return Some(VisitDir::new(e.path()).ok()?);
            }
            None
        }));
        Ok(VisitDir { root, children })
    }

    fn entries(self) -> Box<dyn Iterator<Item = io::Result<DirEntry>>> {
        Box::new(
            self.root
                .chain(self.children.map(|s| s.entries()).flatten()),
        )
    }
}

impl Iterator for VisitDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.root.next() {
            return Some(item);
        }
        if let Some(child) = self.children.next() {
            self.root = child.entries();
            return self.next();
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let paths = VisitDir::new(args.path)?
        .filter_map(|e| Some(e.ok()?.path()))
        .collect::<Vec<_>>();

    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout);
    for path in paths {
        if path.is_file() {
            let mut file = fs::File::open(&path)?;
            let mut hasher = Sha256::new();
            let bytes_written = io::copy(&mut file, &mut hasher)?;
            let hash_bytes = &hasher.finalize();
            writeln!(
                handle,
                "{},{}",
                &path.as_path().display().to_string(),
                HEXLOWER.encode(hash_bytes.as_ref())
            );
        }
    }

    Ok(())
}
