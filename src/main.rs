use std::fs;
use std::env;
use std::fs::File;
use std::io::Read;
use std::hash::Hasher;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::collections::hash_map::DefaultHasher;

#[derive(Debug)]
struct FileEntry {
    inode: u64,
    dev:  u64,
    path : String,
}

fn collect_files(dir: &String, h: &mut HashMap<u64, Vec<FileEntry>>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(path_str) = path.to_str() {
                    if let Ok(metadata) = path.symlink_metadata() {
                        let ft = metadata.file_type();
                        if ft.is_symlink() {
                            continue;
                        } else if ft.is_file() {
                                match metadata.len() {
                                    0 => continue,
                                    i => h.entry(i).or_insert_with(Vec::new)
                                          .push(
                                           FileEntry{ inode : metadata.ino(),
                                               dev   : metadata.dev(),
                                               path  : String::from(path_str),
                                           }),
                                };
                        } else if ft.is_dir() {
                            collect_files(&(String::from(path_str)), h);
                        }
                    }
                }
            }
        }
    } else {
        println!("{}: invalid directory", dir);
    }
}


fn find_duplicates(duplicates: &mut HashMap<u64, Vec<FileEntry>>,
                   vec: &Vec<FileEntry> ) {
    for file_entry in vec {
        if let Ok(mut file) = File::open(&file_entry.path) {
            let mut buf: [u8; 1024] = [0; 1024];
            if let Ok(nbytes) = file.read(&mut buf) {
                let mut hasher = DefaultHasher::new();
                hasher.write(&buf[..nbytes as usize]);
                let k = hasher.finish();
                duplicates.entry(k).or_insert_with(Vec::new)
                    .push(
                        FileEntry{ inode : file_entry.inode,
                            dev  : file_entry.dev,
                            path : file_entry.path.to_string(),
                        });
            }
        }
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();

    // Print out usage if no directories are given.
    if args.len() == 1 {
        println!("Usage: {} <dir1> [dir2 [dir3 ...]]", args[0]);
        ::std::process::exit(0);
    }

    let mut hmap: HashMap<u64, Vec<FileEntry> > = HashMap::new();

    for dir in &args[1..] {
        collect_files(dir, &mut hmap);
    }

    // Get rid of all the single entries.
    hmap.retain(|_, v| v.len() >= 2);


    let mut cluster = 1;

    for (key, val) in hmap.iter() {
        let mut duplicates: HashMap<u64, Vec<FileEntry> > = HashMap::new();
        find_duplicates(&mut duplicates, &val);

        for (hash, vec) in duplicates {
            if vec.len() >= 2 {
                println!("{} files in cluster {} (size: {}, digest: {})",
                         vec.len(), cluster, key, hash);
                for file_entry in vec {
                    println!("{}", file_entry.path);
                }
                cluster += 1;
            }
        }
    }
}
