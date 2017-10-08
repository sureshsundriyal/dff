use std::fs;
use std::env;
use std::fs::File;
use std::io::Read;
use std::hash::Hasher;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::collections::hash_map::DefaultHasher;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize)]
struct FileEntry {
    size : u64,
    hash : u64,
    files : Vec<String>,
}

fn collect_files(dir: &String, files: &mut HashMap<u64, Vec<String>>,
                 inodes: &mut BTreeSet<(u64, u64)>) {

    let entries = match fs::read_dir(dir) {
        Ok(x) => x,
        _ => {
            warn!("{}: invalid directory", dir);
            return
        },
    };

    for path in  entries.filter( |x| x.is_ok() )
                        .map( |x| x.unwrap().path() )
                        .collect::<Vec<PathBuf>>() {
        if let (Some(path_str), Ok(metadata)) =
                (path.to_str(), path.symlink_metadata()) {
            let ft = metadata.file_type();
            if ft.is_symlink() {
                continue;
            } else if ft.is_file() {
                let inode = metadata.ino();
                let device = metadata.dev();
                if inodes.contains(&(inode, device)) {
                    continue;
                }
                inodes.insert((inode, device));
                match metadata.len() {
                    0 => continue, //Ignore empty files.
                    i => files.entry(i).or_insert_with(Vec::new)
                          .push(String::from(path_str)),
                };
            } else if ft.is_dir() {
                collect_files(&(String::from(path_str)), files, inodes);
            }
        } else {
            warn!("Failed to retrieve metadata for {}", path.to_str().unwrap());
        }
    }
}

fn find_duplicates(duplicates: &mut HashMap<u64, Vec<String>>,
                   vec: &Vec<String>, thorough: bool ) {
    for file_entry in vec {
        if let Ok(mut file) = File::open(file_entry) {
            let mut hash: u64 = 0;
            let mut hasher = DefaultHasher::new();
            if thorough {
                let mut contents: Vec<u8> = Vec::new();
                if let Ok(_) = file.read_to_end(&mut contents) {
                    hasher.write(&contents[..]);
                    hash = hasher.finish();
                } else {
                    warn!("Failed to read {}", file_entry);
                }
            } else {
                let mut buf: [u8; 1024] = [0; 1024];
                if let Ok(nbytes) = file.read(&mut buf) {
                    hasher.write(&buf[..nbytes as usize]);
                    hash = hasher.finish();
                } else {
                    warn!("Failed to read {} chunk", file_entry);
                }
            }
            if hash != 0 {
                duplicates.entry(hash).or_insert_with(Vec::new)
                    .push(file_entry.to_string());
            }
        } else {
            warn!("Failed to open file {}", file_entry);
        }
    }
}

fn print_duplicates(vec: &Vec<String>, cluster: i32, key: u64, hash: u64,
                    json_list: &mut Vec<FileEntry>, json_output: bool) {
    if json_output {
        json_list.push( FileEntry {
            size  : key,
            hash  : hash,
            files : vec.to_vec(),
        });
    } else {
        println!("{} files in cluster {} (size: {}, digest: {})",
                 vec.len(), cluster, key, hash);
        // for_each becomes stable v1.22.0 onwards. Should uncomment then.
        //vec.iter().for_each(|f| println!("{}", f.path));
        for file_entry in vec {
            println!("{}", file_entry);
        }
    }
}

fn exhaustive_search(emap: &mut HashMap<Vec<u8>, Vec<String>>,
                     vec: &Vec<String>) {
    for file_entry in vec {
        if let Ok(mut file) = File::open(file_entry) {
            let mut contents: Vec<u8> = Vec::new();
            if let Ok(_) = file.read_to_end(&mut contents) {
                emap.entry(contents).or_insert_with(Vec::new)
                    .push(file_entry.to_string());
            } else {
                warn!("Failed to read {}", file_entry);
            }
        } else {
            warn!("Failed to open {}", file_entry);
        }
    }
}

fn main() {
    env_logger::init().unwrap();

    let mut args: Vec<String> = env::args().collect();

    let binary_name = args.remove(0);

    let mut hmap: HashMap<u64, Vec<String> > = HashMap::new();
    let mut bset: BTreeSet<(u64, u64)> = BTreeSet::new();

    let mut thorough = false;
    let mut exhaustive = false;
    let mut json_output = false;
    let mut print_usage = true;

    for dir in &args[..] {
        match dir.as_ref() {
            "-t" => {
                thorough = true;
                continue;
            },
            "-e" => {
                thorough = false;
                exhaustive = false;
                continue;
            },
            "-j" => {
                json_output = true;
                continue;
            },
            _ => {
                print_usage = false;
                collect_files(dir, &mut hmap, &mut bset);
            },
        }
    }

    if print_usage {
        println!("Usage: {} [-t] [-e] [-j] <dir1> [dir2 [dir3 ...]]",
                 binary_name);
        ::std::process::exit(0);
    }

    // Get rid of all the single entries.
    hmap.retain(|_, v| v.len() >= 2);

    let mut cluster = 1;

    let mut json_list : Vec<FileEntry> = Vec::new();
    for (key, val) in hmap {
        let mut duplicates: HashMap<u64, Vec<String> > = HashMap::new();
        find_duplicates(&mut duplicates, &val, thorough);

        for (hash, vec) in duplicates {
            if vec.len() >= 2 {
                if exhaustive {
                    let mut emap: HashMap<Vec<u8>, Vec<String>> = HashMap::new();
                    exhaustive_search(&mut emap, &vec);
                    for (_, v) in emap {
                        if v.len() >= 2 {
                            print_duplicates(&v, cluster, key, hash,
                                             &mut json_list, json_output);
                            cluster += 1;
                        }
                    }
                } else {
                    print_duplicates(&vec, cluster, key, hash, &mut json_list,
                                     json_output);
                    cluster += 1;
                }
            }
        }
    }
    if json_output {
        println!("{}", serde_json::to_string_pretty(&json_list).unwrap());
    }
}
