#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod debounce;
mod file;
mod lang;
mod snippet;
mod watch;

use file::{open_file, FileReader};
use snippet::KeyList;
use std::thread;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    snippets_dir: String,
    dirs: Vec<String>,
    files: Vec<String>,
}

const DEBOUNCE_INTERVAL: u64 = 10_000; // ms

fn make_snippet(snippets_dir: &str, code_filepath_string: &String) {
    let code_filepath = std::path::PathBuf::from(code_filepath_string);

    if let Some(extension) = file::get_extension(&code_filepath) {
        // 拡張子から言語を特定する
        let lang_identifier = lang::get_lang(extension);
        if lang_identifier.is_none() {
            return;
        }
        let lang_identifier = lang_identifier.unwrap();

        make(lang_identifier, snippets_dir, &code_filepath);
        return;
    }
}

fn main() {
    let mut config_path: std::path::PathBuf;

    config_path = std::env::current_exe().expect("cannot get current_exe");
    config_path.pop();
    config_path.push("config.json");
    println!("Config: {:?}", config_path);

    let contents = file::read_file(&config_path);
    let config: Config = serde_json::from_str(&contents).expect("cannot perse config.json");
    let paths = [config.dirs.clone(), config.files.clone()].concat();

    let debounce_interval = std::time::Duration::from_millis(DEBOUNCE_INTERVAL);
    let debouncer = debounce::get_safe_debouncer(debounce_interval);
    let snippets_dir = config.snippets_dir.clone();

    // TODO: configも監視しておく
    // 監視する
    let register_result = watch::watch_dir(paths, |code_filepath| {
        // TODO: ファイルごとにdebounceする
        let locked = debouncer.lock();
        if let Ok(_) = locked {
            // debounceに注意
            let run = locked.unwrap().debounce(|| {
                make_snippet(snippets_dir.clone().as_str(), &code_filepath);
            });

            // これが最後のmake_snippetだった場合、debounce_interval間に起こる編集イベントに対応できない
            // → 常にdebounce_interval後にファイルの編集を確認しに行く
            if run {
                let snippets_dir_string = snippets_dir.clone();
                thread::spawn(move || {
                    thread::sleep(debounce_interval);
                    make_snippet(snippets_dir_string.as_str(), &code_filepath);
                });
            }
        }
    });
    match register_result {
        Ok(()) => (),
        Err(_) => {
            panic!("cannot watch files!");
        }
    };
}

fn make(lang_identifier: String, snippets_dir: &str, code_filepath: &std::path::PathBuf) {
    // スニペットを切り出す
    let snippet_file = open_file(&code_filepath, false, false);
    if snippet_file.is_none() {
        return;
    }

    let snippet_reader = FileReader::new(snippet_file.unwrap());

    let code_filepath_string = std::path::PathBuf::from(code_filepath)
        .into_os_string()
        .into_string()
        .clone()
        .unwrap();

    // 現存してるスニペット情報を取得する + コードの削除をチェック
    let list_filepath = snippet::get_namelist_filepath(lang_identifier.as_str(), snippets_dir);
    let list_file = open_file(&list_filepath, true, false);
    if list_file.is_none() {
        return;
    }

    let mut list_file_reader = FileReader::new(list_file.unwrap());

    let snippet_json_filename = format!("{}.json", lang_identifier);
    let mut snippet_json_filepath = std::path::PathBuf::from(snippets_dir);

    snippet_json_filepath.push(snippet_json_filename);
    println!("{:?}", snippet_json_filepath);

    if let Some(snippet_json_file) = open_file(&snippet_json_filepath, true, false) {
        let snippet_json_reader = FileReader::new(snippet_json_file);
        let result = snippet::make(
            snippet_reader,
            snippet_json_reader,
            &mut list_file_reader,
            code_filepath_string,
        );
        if let Some(result) = result {
            // スニペットのjsonを書き込む
            file::write_file(&snippet_json_filepath, result.json);

            // 新しいnamelistを書き込む
            if let Ok(name_list_string) = serde_json::to_string::<KeyList>(&result.name_list) {
                file::write_file(&list_filepath, name_list_string);
            }
        }
    }
}
