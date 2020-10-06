#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod file;
mod lang;
mod snippet;
mod watch;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    snippets_dir: String,
    dirs: Vec<String>,
    files: Vec<String>,
}

fn receive_event(config: &Config, code_filepath: &std::path::PathBuf) {
    if let Some(extension) = file::get_extension(&code_filepath) {
        // 拡張子から言語を特定する
        let lang_identifier = lang::get_lang(extension);
        if lang_identifier.is_none() {
            return;
        }

        let lang_identifier = lang_identifier.unwrap();
        let snippets_dir = String::from(&config.snippets_dir);
        // println!("{:?}", lang_name);

        snippet::make(lang_identifier, snippets_dir, &code_filepath);
        return;
    }
}

fn main() {
    let mut config_path = std::env::current_exe().expect("cannot get current_exe"); // 実行ファイルの置かれてるファイルパス

    config_path.pop();
    config_path.push("config.json");
    println!("Config: {:?}", config_path);

    let contents = file::read_file(&config_path);
    let config: Config = serde_json::from_str(&contents).expect("cannot perse config.json");
    let paths = [config.dirs.clone(), config.files.clone()].concat();

    // TODO: configも監視しておく
    // 監視する
    match watch::watch_dir(paths, &config, receive_event) {
        Ok(()) => (),
        Err(_) => {
            panic!("cannot watch files");
        }
    };
}
