use std::fs::File;
use std::io::prelude::*;

pub fn read_file(path: &std::path::PathBuf) -> String {
    let mut contents = String::new();
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            panic!(format!("file not found. Check {}", path.to_str().unwrap()));
        }
    };

    file.read_to_string(&mut contents)
        .expect("something went wrong reading the config file");
    return contents;
}

pub fn write_file(path: &std::path::PathBuf, text: String) {
    let file = match std::fs::OpenOptions::new().write(true).open(path) {
        Ok(f) => f,
        Err(_) => {
            panic!(format!("file not found. Check {}", path.to_str().unwrap()));
        }
    };

    println!("{:?}", file);
    write!(&file, "{}", text);
}

pub fn get_extension(path: &std::path::PathBuf) -> Option<String> {
    // 拡張子を取得
    let filename_osstr = path.file_name();
    if filename_osstr.is_none() {
        return None;
    }

    let filename_str = filename_osstr.unwrap().to_str();
    if filename_str.is_none() {
        return None;
    }

    let filename = String::from(filename_str.unwrap());
    if let Some(extension) = filename.split(".").last() {
        return Some(String::from(extension));
    }

    return None;
}
