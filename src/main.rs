use std::fs;
use std::env;
use std::fs::File;
use std::io::Read;
use std::hash::Hasher;
use std::path::PathBuf;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::collections::hash_map::DefaultHasher;

#[macro_use]
extern crate log;
extern crate env_logger;

struct FileEntry {
    inode: u64,
    dev:  u64,
    path : String,
}

fn collect_files(dir: &String, h: &mut HashMap<u64, Vec<FileEntry>>) {

    let entries = match fs::read_dir(dir) {
        Ok(x) => x,
        _ => {
            warn!("{}: invalid directory", dir);
            return
        },
    };

    for path in  entries.filter( |x| x.is_ok() ).map( |x| x.unwrap().path() )
                .collect::<Vec<PathBuf>>() {
        if let (Some(path_str), Ok(metadata)) =
                (path.to_str(), path.symlink_metadata()) {
            let ft = metadata.file_type();
            if ft.is_symlink() {
                continue;
            } else if ft.is_file() {
                match metadata.len() {
                    0 => continue, //Ignore empty files.
                    i => h.entry(i).or_insert_with(Vec::new)
                          .push(
                           FileEntry{
                               inode : metadata.ino(),
                               dev   : metadata.dev(),
                               path  : String::from(path_str),
                           }),
                };
            } else if ft.is_dir() {
                collect_files(&(String::from(path_str)), h);
            }
        } else {
            warn!("Failed to retrieve metadata for {}", path.to_str().unwrap());
        }
    }
}

fn find_duplicates(duplicates: &mut HashMap<u64, Vec<FileEntry>>,
                   vec: &Vec<FileEntry>, thorough: bool ) {
    for file_entry in vec {
        if let Ok(mut file) = File::open(&file_entry.path) {
            let mut hash: u64 = 0;
            let mut hasher = DefaultHasher::new();
            if thorough {
                let mut contents: Vec<u8> = Vec::new();
                if let Ok(_) = file.read_to_end(&mut contents) {
                    hasher.write(&contents[..]);
                    hash = hasher.finish();
                } else {
                    warn!("Failed to read {}", file_entry.path);
                }
            } else {
                let mut buf: [u8; 1024] = [0; 1024];
                if let Ok(nbytes) = file.read(&mut buf) {
                    hasher.write(&buf[..nbytes as usize]);
                    hash = hasher.finish();
                } else {
                    warn!("Failed to read {} chunk", file_entry.path);
                }
            }
            if hash != 0 {
                duplicates.entry(hash).or_insert_with(Vec::new)
                    .push(
                        FileEntry{
                            inode : file_entry.inode,
                            dev   : file_entry.dev,
                            path  : file_entry.path.to_string(),
                        });
            }
        } else {
            warn!("Failed to open file {}", file_entry.path);
        }
    }
}

fn main() {
    env_logger::init().unwrap();

    let args: Vec<String> = env::args().collect();

    // Print out usage if no directories are given.
    if args.len() == 1 || ( args.len() == 2 && args[1] == "-t" ) {
        println!("Usage: {} [-t] <dir1> [dir2 [dir3 ...]]", args[0]);
        ::std::process::exit(0);
    }

    let mut hmap: HashMap<u64, Vec<FileEntry> > = HashMap::new();

    let mut thorough = false;
    for dir in &args[1..] {
        if dir == "-t" {
            thorough = true;
            continue;
        }
        collect_files(dir, &mut hmap);
    }

    // Get rid of all the single entries.
    hmap.retain(|_, v| v.len() >= 2);

    let mut cluster = 1;

    for (key, val) in hmap.iter() {
        let mut duplicates: HashMap<u64, Vec<FileEntry> > = HashMap::new();
        find_duplicates(&mut duplicates, &val, thorough);

        for (hash, vec) in duplicates {
            if vec.len() >= 2 {
                println!("{} files in cluster {} (size: {}, digest: {})",
                         vec.len(), cluster, key, hash);
                // for_each becomes stable v1.22.0 onwards. Should uncomment then.
                //vec.iter().for_each(|f| println!("{}", f.path));
                for file_entry in vec {
                    println!("{}", file_entry.path);
                }
                cluster += 1;
            }
        }
    }
}
