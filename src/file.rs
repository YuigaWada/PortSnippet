use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

pub trait Reader {
    fn lines(&self) -> Vec<String>;
    fn all(&mut self) -> String;
}

pub struct FileReader {
    file: File,
}

impl FileReader {
    pub fn new(file: File) -> FileReader {
        return FileReader { file: file };
    }
}

impl Reader for FileReader {
    fn lines(&self) -> Vec<String> {
        let mut result = vec![];
        for line in BufReader::new(&self.file).lines() {
            let line = line.unwrap();
            result.push(line);
        }

        return result;
    }

    fn all(&mut self) -> String {
        let mut contents = String::new();
        self.file
            .read_to_string(&mut contents)
            .expect("something went wrong reading the config file");
        return contents;
    }
}

pub fn open_file(path: &std::path::PathBuf, create: bool, should_panic: bool) -> Option<File> {
    let file = match std::fs::OpenOptions::new()
        .create(create)
        .read(true)
        .write(true)
        .open(path)
    {
        Ok(f) => f,
        Err(e) => {
            if should_panic {
                panic!(format!("something went wrong: {}", e));
            } else {
                return None;
            }
        }
    };

    return Some(file);
}

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
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
    {
        Ok(f) => f,
        Err(_) => {
            panic!(format!("file not found. Check {}", path.to_str().unwrap()));
        }
    };

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
