use std::fs;
use std::env;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;
use std::io::Read;

#[derive(Debug)]
struct FileEntry {
    inode: u64,
    size: u64,
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
                        if !ft.is_symlink() {
                            if ft.is_file() {
                                let file_size = metadata.len();
                                if !h.contains_key(&file_size) {
                                    h.insert(file_size, Vec::new());
                                }
                                if let Some(vec) = h.get_mut(&file_size) {
                                    vec.push(
                                         FileEntry{ inode : metadata.ino(),
                                             size  : metadata.len(),
                                             dev   : metadata.dev(),
                                             path  : String::from(path_str),
                                         });
                                }
                            } else if ft.is_dir() {
                                collect_files(&(String::from(path_str)), h);
                            }
                        }
                    }
                }
            }
        }
    }
    else {
        println!("{}: invalid directory", dir);
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

    let mut duplicates: HashMap<(u64, u64), Vec<FileEntry> > = HashMap::new();

    for (key, val) in hmap.iter() {
        for file_entry in val {
            if let Ok(mut file) = fs::File::open(&file_entry.path) {
                let mut buf: [u8; 1024] = [0; 1024];
                if let Ok(nbytes) = file.read(&mut buf) {
                    let mut hasher = DefaultHasher::new();
                    hasher.write(&buf[..nbytes as usize]);
                    let k = (*key, hasher.finish());
                    if !duplicates.contains_key(&k) {
                        duplicates.insert(k, Vec::new());
                    }
                    if let Some(vec) = duplicates.get_mut(&k) {
                        vec.push(
                            FileEntry{ inode : file_entry.inode,
                                size : file_entry.size,
                                dev  : file_entry.dev,
                                path : file_entry.path.to_string(),
                            });
                    }
                }
            }
        }
    }

    duplicates.retain(|_, v| v.len() >= 2);
    println!("Duplicates: {:?}", duplicates);
}
