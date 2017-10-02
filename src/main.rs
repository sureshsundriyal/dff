use std::fs;
use std::env;
use std::os::unix::fs::MetadataExt;

#[derive(Debug)]
struct File {
    inode: u64,
    size: u64,
    path : String,
}

fn collect_files(dirs: &[String], v: &mut Vec<File>, d: &mut Vec<String>) {
    for dir in dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
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
                            d.push(String::from(path_str));
                        }
                    }
                }
            }
        }
        else {
            println!("{}: invalid directory", dir);
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

    let mut a = (args[1..]).to_vec();
    let mut v: Vec<File>  = Vec::new();
    loop {
        let mut d: Vec<String>   = Vec::new();
        collect_files(&a[..], &mut v, &mut d);
        if d.len() > 0 {
            a = (d[..]).to_vec();
        } else {
            break;
        }
    }
    println!("{:?}", v);
}
