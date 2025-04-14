use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};  // Pour obtenir les métadonnées et les permissions
use chrono::{DateTime, Local};
use users::{get_user_by_uid, get_group_by_gid};

fn main() {
    loop {
        let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        print!("\n{}$", current_dir.display());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        let trimmed = input.trim();
        if trimmed == "exit" {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        let args: Vec<&str> = trimmed.split_whitespace().collect();

        match args[0] {
            "echo" => {
                if args.len() > 1 {
                    println!("{}", args[1..].join(" "));
                }
            },
            "pwd" => {
                match env::current_dir() {
                    Ok(dir) => println!("{}", dir.display()),
                    Err(e) => eprintln!("pwd: error: {}", e),
                }
            },
            "ls" => {
                let mut show_hidden = false;
                let mut long_format = false;
                let mut classify = false;
                
                for arg in &args[1..] {
                    match *arg {
                        "-a" => show_hidden = true,
                        "-l" => long_format = true,
                        "-F" => classify = true,
                        _ => {}
                    }
                }
                
                if let Err(e) = list_files("./", show_hidden, long_format, classify) {
                    eprintln!("ls: error: {}", e);
                }
            },
            "cd" => {
                if args.len() > 1 {
                    if let Err(e) = env::set_current_dir(args[1]) {
                        eprintln!("cd: {}: {}", args[1], e);
                    }
                } else {
                    eprintln!("cd: missing argument");
                }
            },
            "cat" => {
                if args.len() > 1 {
                    let filename = args[1];
                    match fs::read_to_string(filename) {
                        Ok(contents) => print!("{}", contents),
                        Err(e) => eprintln!("cat: {}: {}", filename, e),
                    }
                } else {
                    eprintln!("cat: missing file argument");
                }
            },
            "cp" => {
                if args.len() > 2 {
                    let source = args[1];
                    let destination = args[2];

                    let metadata = fs::metadata(source);
                    if metadata.is_err() || !metadata.unwrap().is_file() {
                        eprintln!("cp: error: {} is not a valid file", source);
                        continue;
                    }

                    let dest_path = std::path::Path::new(destination);

                    let final_destination = if dest_path.is_dir() {
                        dest_path.join(std::path::Path::new(source).file_name().unwrap())
                    } else {
                        dest_path.to_path_buf()
                    };

                    match fs::copy(source, final_destination) {
                        Ok(_) => {},
                        Err(e) => eprintln!("cp: error: {}", e),
                    }
                } else {
                    eprintln!("cp: missing source or destination");
                }
            },
            "rm" => {
                if args.len() > 1 {
                    if args[1] == "-r" && args.len() > 2 {
                        let dir = args[2];
                        match fs::remove_dir_all(dir) {
                            Ok(_) => {},
                            Err(e) => eprintln!("rm: error: {}", e),
                        }
                    } else {
                        let file = args[1];
                        match fs::remove_file(file) {
                            Ok(_) => {},
                            Err(e) => eprintln!("rm: error: {}", e),
                        }
                    }
                } else {
                    eprintln!("rm: missing file argument");
                }
            },
            "mv" => {
                if args.len() > 2 {
                    let source = args[1];
                    let destination = args[2];
                    match fs::rename(source, destination) {
                        Ok(_) => {},
                        Err(e) => eprintln!("mv: error: {}", e),
                    }
                } else {
                    eprintln!("mv: missing source or destination");
                }
            },
            "mkdir" => {
                if args.len() > 1 {
                    match fs::create_dir(args[1]) {
                        Ok(_) => {},
                        Err(e) => eprintln!("mkdir: error: {}", e),
                    }
                } else {
                    eprintln!("mkdir: missing directory argument");
                }
            },
            _ => eprintln!("Command not found: {}", args[0]),
        }
    }
}

fn list_files(path: &str, show_hidden: bool, long_format: bool, classify: bool) -> std::io::Result<()> {
    let entries = fs::read_dir(path)?;
    let mut files = vec![];

    // Inclure "." et ".." dans la liste
    files.push((".".to_string(), fs::metadata(".")?));
    files.push(("..".to_string(), fs::metadata("..")?));
    
    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        
        if !show_hidden && file_name_str.starts_with('.') {
            continue;
        }
        
        files.push((file_name_str.to_string(), metadata));
    }
    
    let total_blocks: u64 = files.iter().map(|(_, meta)| meta.blocks()).sum();
    println!("total {}", total_blocks/2);
    
    for (file_name, metadata) in files {
        let mut display_name = file_name.clone();
        
        if classify {
            if metadata.is_dir() {
                display_name.push('/');
            } else if metadata.permissions().mode() & 0o111 != 0 {
                display_name.push('*');
            }
        }
        
        if long_format {
            let permissions = format_permissions(metadata.permissions().mode());
            let nlinks = metadata.nlink();
            let user = get_user_by_uid(metadata.uid()).map_or("?".to_string(), |u| u.name().to_string_lossy().into_owned());
            let group = get_group_by_gid(metadata.gid()).map_or("?".to_string(), |g| g.name().to_string_lossy().into_owned());
            let size = metadata.size();
            let modified: DateTime<Local> = DateTime::from(metadata.modified().unwrap());
            let formatted_date = modified.format("%b %d %H:%M").to_string();
            
            println!("{} {:>2} {} {} {:>8} {} {}", permissions, nlinks, user, group, size, formatted_date, display_name);
        } else {
            println!("{}", display_name);
        }
    }
    Ok(())
}

fn format_permissions(mode: u32) -> String {
    let user = (mode & 0o700) >> 6;
    let group = (mode & 0o070) >> 3;
    let others = mode & 0o007;

    let mut permissions = String::new();
    permissions.push(if mode & 0o40000 != 0 { 'd' } else { '-' });
    permissions.push(if user & 4 != 0 { 'r' } else { '-' });
    permissions.push(if user & 2 != 0 { 'w' } else { '-' });
    permissions.push(if user & 1 != 0 { 'x' } else { '-' });
    permissions.push(if group & 4 != 0 { 'r' } else { '-' });
    permissions.push(if group & 2 != 0 { 'w' } else { '-' });
    permissions.push(if group & 1 != 0 { 'x' } else { '-' });
    permissions.push(if others & 4 != 0 { 'r' } else { '-' });
    permissions.push(if others & 2 != 0 { 'w' } else { '-' });
    permissions.push(if others & 1 != 0 { 'x' } else { '-' });
    permissions
}
