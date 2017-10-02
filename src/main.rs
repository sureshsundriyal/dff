use std::fs;
use std::env;
use std::os::unix::fs::MetadataExt;

#[derive(Debug)]
struct File {
    inode: u64,
    size: u64,
    path : String,
}

fn collect_files(dir: &String, v: &mut Vec<File>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(path_str) = path.to_str() {
                    if path.is_file() {
                        if let Ok(metadata) = entry.metadata() {
                            v.push(File{ inode : metadata.ino(),
                                         size  : metadata.len(),
                                         path  : String::from(path_str),
                                    })
                        }
                    } else if path.is_dir() {
                        collect_files(&(String::from(path_str)), v);
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

    let mut v: Vec<File>  = Vec::new();

    for dir in &args[1..] {
        collect_files(dir, &mut v);
    }
    println!("{:?}", v);
}
