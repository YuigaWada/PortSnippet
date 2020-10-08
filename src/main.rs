#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod debounce;
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

const DEBOUNCE_INTERVAL: u64 = 6000; // ms

fn make_snippet(config: &Config, code_filepath_string: &String) {
    let snippets_dir = String::from(&config.snippets_dir);
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

    // TODO: configも監視しておく
    // 監視する
    let register_result = watch::watch_dir(paths, |code_filepath| { 
        let locked = debouncer.lock();
        if let Ok(_) = locked {
            locked.unwrap().debounce(|| { // debounceに注意
                make_snippet(&config, &code_filepath); 
            });
        }
    });
    match register_result {
        Ok(()) => (),
        Err(_) => {
            panic!("cannot watch files!");
        }
    };
}
