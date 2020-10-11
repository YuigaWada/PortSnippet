#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod debounce;
mod file;
mod lang;
mod snippet;
mod watch;

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

        snippet::make(lang_identifier, snippets_dir, &code_filepath);
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
