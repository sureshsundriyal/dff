use std::fs;
use std::env;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;

#[derive(Debug)]
struct File {
    inode: u64,
    size: u64,
    dev:  u64,
    path : String,
}

fn collect_files(dir: &String, h: &mut HashMap<u64, Vec<File>>) {
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
                                         File{ inode : metadata.ino(),
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

    let mut hmap: HashMap<u64, Vec<File> > = HashMap::new();

    for dir in &args[1..] {
        collect_files(dir, &mut hmap);
    }

    // Get rid of all the single entries.
    hmap.retain(|_, v| v.len() >= 2);
    println!("{:?}", hmap);
}
