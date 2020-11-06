#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod daemon;
mod debounce;
mod file;
mod lang;
mod snippet;
mod watch;

use file::{open_file, FileReader};
use snippet::KeyList;
use std::thread;

const DEBOUNCE_INTERVAL: u64 = 10_000; // ms

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    snippets_dir: String,
    dirs: Vec<String>,
    files: Vec<String>,
}

enum LaunchType {
    Man,     // 直接人間が起動させた場合
    Daemon,  // cron処理でpcが起動させた場合
    Stop,    // PortSnippetを停止する
    Restart, // PortSnippetを再起動する
    Help,    // help
}

struct Params {
    man: String,
    daemon: String,
    stop: String,
    restart: String,
    help: String,
}

const AUTO_LAUNCH_PARAM: &str = "AUTO_LAUNCH";
const MAN_PARAM: &str = "man";
const STOP_PARAM: &str = "stop";
const RESTART_PARAM: &str = "restart";
const HELP_PARAM: &str = "help";

#[cfg(debug_assertions)] // デバッグ用
fn check_type() -> LaunchType {
    return LaunchType::Daemon;
}

#[cfg(not(debug_assertions))] // リリース用
fn check_type() -> LaunchType {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return LaunchType::Man;
    }
    if args[1] == AUTO_LAUNCH_PARAM {
        return LaunchType::Daemon;
    }

    for i in 0..2 {
        let short = i == 0;
        let params = get_params(short);
        let man = params.man.as_str();
        let stop = params.stop.as_str();
        let restart = params.restart.as_str();
        let help = params.help.as_str();

        if &args[1] == man {
            return LaunchType::Daemon;
        } else if &args[1] == stop {
            return LaunchType::Stop;
        } else if &args[1] == restart {
            return LaunchType::Restart;
        } else if &args[1] == help {
            return LaunchType::Help;
        }
    }

    return LaunchType::Man;
}

fn get_params(short: bool) -> Params {
    let mut man = MAN_PARAM.to_string();
    let mut daemon = AUTO_LAUNCH_PARAM.to_string();
    let mut stop = STOP_PARAM.to_string();
    let mut restart = RESTART_PARAM.to_string();
    let mut help = HELP_PARAM.to_string();

    if short {
        man = format!("-{}", man.chars().take(1).collect::<String>());
        daemon = format!("-{}", daemon.chars().take(1).collect::<String>());
        stop = format!("-{}", stop.chars().take(1).collect::<String>());
        restart = format!("-{}", restart.chars().take(1).collect::<String>());
        help = format!("-{}", help.chars().take(1).collect::<String>());
    }

    return Params {
        man: man,
        daemon: daemon,
        stop: stop,
        restart: restart,
        help: help,
    };
}

fn print_help() {}

fn main() {
    let launch_type = check_type();

    match launch_type {
        LaunchType::Daemon => {
            watch();
        }
        LaunchType::Man => {
            // cronの登録処理
            println!("Registering daemon...\n");
            daemon::register();

            let messages = daemon::get_complete_messages();
            println!("{}", messages);
        }
        LaunchType::Stop => {
            daemon::stop();
            println!("stop");
        }
        LaunchType::Restart => {
            daemon::stop();
            daemon::run();
            println!("restart!");
        }
        LaunchType::Help => {
            print_help();
        }
    }
}

// フォルダ・ファイルを監視
fn watch() {
    let mut config_path: std::path::PathBuf;

    config_path = std::env::current_exe().expect("cannot get current_exe");
    config_path.pop();
    config_path.push("config.json");
    println!("Config: {:?}", config_path);

    let contents = file::read_file(&config_path);
    let config: Config = serde_json::from_str(&contents).expect("cannot perse config.json");
    let paths = [config.dirs.clone(), config.files.clone()].concat();

    let mut debouncers = debounce::SafeFileDebouncer::new(DEBOUNCE_INTERVAL); // ファイルごとにdebounceする
    let snippets_dir = config.snippets_dir.clone();

    // TODO: configも監視しておく
    // 監視する
    let register_result = watch::watch_dir(paths, |code_filepath_string| {
        let debouncer = debouncers.get(code_filepath_string.as_str()); // ファイルに紐付いたdebouncerを取り出す
        let locked = debouncer.lock();
        let code_filepath = std::path::PathBuf::from(code_filepath_string);

        if let Ok(_) = locked {
            let run = locked.unwrap().debounce(|| {
                make_snippet(snippets_dir.clone().as_str(), &code_filepath);
            });

            // これが最後のmake_snippetだった場合、debounce_interval間に起こる編集イベントに対応できない
            // → 常にdebounce_interval後にファイルの編集を確認しに行く
            if run {
                let snippets_dir_string = snippets_dir.clone();
                thread::spawn(move || {
                    let debounce_interval = std::time::Duration::from_millis(DEBOUNCE_INTERVAL);
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

// ファイルの拡張子から言語を特定する
fn detect_lang(code_filepath: &std::path::PathBuf) -> Option<String> {
    if let Some(extension) = file::get_extension(&code_filepath) {
        let lang_identifier = lang::get_lang(extension);
        return lang_identifier;
    }
    return None;
}

// スニペットを生成
fn make_snippet(snippets_dir: &str, code_filepath: &std::path::PathBuf) {
    // 言語の特定 / 対象ファイルの読み込み
    let lang_identifier = detect_lang(code_filepath);
    let snippet_file = open_file(&code_filepath, false, false);
    if lang_identifier.is_none() || snippet_file.is_none() {
        return;
    }

    let lang_identifier = lang_identifier.unwrap();
    let code_filepath_string = std::path::PathBuf::from(code_filepath)
        .into_os_string()
        .into_string()
        .clone()
        .unwrap();

    // namelistの読み込み
    let list_filepath = snippet::get_namelist_filepath(lang_identifier.as_str(), snippets_dir);
    let list_file = open_file(&list_filepath, true, false);
    if list_file.is_none() {
        return;
    }

    // スニペットのjsonファイルを読み込む
    let snippet_json_filename = format!("{}.json", lang_identifier);
    let mut snippet_json_filepath = std::path::PathBuf::from(snippets_dir);
    snippet_json_filepath.push(snippet_json_filename);
    println!("{:?}", snippet_json_filepath);

    // スニペットを生成
    if let Some(snippet_json_file) = open_file(&snippet_json_filepath, true, false) {
        // Readerを用意する
        let snippet_reader = FileReader::new(snippet_file.unwrap());
        let mut list_file_reader = FileReader::new(list_file.unwrap());
        let snippet_json_reader = FileReader::new(snippet_json_file);

        // make!
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
