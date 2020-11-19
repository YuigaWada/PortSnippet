use super::super::file;
use std::path::PathBuf;
use std::process::Command;

const EXE_VARIABLE: &str = "{{EXE_PATH}}";
const LOG_VARIABLE: &str = "{{LOG_FILE_PATH}}";
const ERROR_LOG_VARIABLE: &str = "{{ERROR_LOG_FILE_PATH}}";

const PLIST_FILENAME: &str = "launch-port-snippet";
const PLIST_FILEPATH: &str = "~/Library/LaunchAgents/launch-port-snippet.plist";
const PLIST_TEMPLATE: &str = r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>launch-port-snippet</string>
    <key>ProgramArguments</key>
    <array>
        <string>{{EXE_PATH}}</string>
        <string>AUTO_LAUNCH</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{{LOG_FILE_PATH}}</string>
    <key>StandardErrorPath</key>
    <string>{{ERROR_LOG_FILE_PATH}}</string>
</dict>
</plist>
"#;

struct LogPaths {
    standard: PathBuf,
    error: PathBuf,
}

// ログファイルのパスを取得する
fn get_log_path(exe_path: &PathBuf) -> LogPaths {
    let mut log_dir = exe_path.clone();
    log_dir.pop();
    log_dir.push(".log");

    let mut log_path = log_dir.clone();
    log_path.push("standard.log"); // > ./(exe)/.log/standard.log

    let mut error_log_path = log_dir.clone();
    error_log_path.push("error.log"); // > ./(exe)/.log/error.log

    return LogPaths {
        standard: log_path,
        error: error_log_path,
    };
}

// plistに変数を注入する
fn inject_variables(mut plist: String, exe_path: PathBuf) -> String {
    let log_paths = get_log_path(&exe_path);
    let exe_path_string = exe_path
        .into_os_string()
        .into_string()
        .expect("cannot get current_exe");

    plist = plist.replace(EXE_VARIABLE, &exe_path_string);
    plist = plist.replace(
        LOG_VARIABLE,
        &log_paths
            .standard
            .to_str()
            .expect("something went wrong: cannot convert PathBuf → str"),
    );
    plist = plist.replace(
        ERROR_LOG_VARIABLE,
        &log_paths
            .error
            .to_str()
            .expect("something went wrong: cannot convert PathBuf → str"),
    );

    return plist;
}

// launchdを操作する
fn operate_launchd(mode: &str, arg: &str) {
    let launchctl = Command::new("launchctl")
        .arg(mode)
        .arg(arg)
        .output()
        .expect("cannot run launchctl");
    let result_message = launchctl.stdout;
    println!("{}\n...\n", std::str::from_utf8(&result_message).unwrap());
}

// daemonを登録
pub fn register(need_run: bool) {
    let home_dir = std::env::var("HOME").unwrap();
    let exe_path = std::env::current_exe().expect("cannot get current_exe");

    let mut plist = PLIST_TEMPLATE.to_string();
    plist = inject_variables(plist, exe_path);

    let plist_filepath_string = PLIST_FILEPATH.replace("~", &home_dir);
    let plist_filepath = PathBuf::from(&plist_filepath_string);
    file::write_file(&plist_filepath, plist);

    println!("> launchctl load {}", &plist_filepath_string);
    if need_run {
        run();
    }
}

// 完了メッセージ
pub fn get_complete_messages() -> String {
    return format!(
        "{}\n{}\n\nA plist file is saved as \"{}\"!\n",
        "Daemon setup is now completed!",
        "When you logs in a new launchd, PortSnippet process will be started by launchd.",
        PLIST_FILEPATH
    );
}

// PortSnippetをlaunchd経由で起動する
pub fn run() {
    let home_dir = std::env::var("HOME").unwrap();
    let plist_filepath_string = PLIST_FILEPATH.replace("~", &home_dir);
    operate_launchd("unload", plist_filepath_string.as_str());
    operate_launchd("load", plist_filepath_string.as_str());
}

// PortSnippetを停止する
pub fn stop() {
    operate_launchd("stop", PLIST_FILENAME);
}
