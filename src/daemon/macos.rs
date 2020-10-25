use super::super::file;
use std::path::PathBuf;
use std::process::Command;

const EXE_VARIABLE: &str = "{{ABSOLUTE_PATH}}";

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
        <string>{{ABSOLUTE_PATH}}</string>
        <string>AUTO_LAUNCH</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#;

pub fn register() {
    let home_dir = std::env::var("HOME").unwrap();
    let exe_path = std::env::current_exe().expect("cannot get current_exe");
    let exe_path_string = exe_path
        .into_os_string()
        .into_string()
        .expect("cannot get current_exe");

    let mut plist = PLIST_TEMPLATE.to_string();
    plist = plist.replace(EXE_VARIABLE, &exe_path_string);

    let plist_filepath_string = PLIST_FILEPATH.replace("~", &home_dir);
    let plist_filepath = PathBuf::from(&plist_filepath_string);
    file::write_file(&plist_filepath, plist);

    println!("> launchctl load {}",&plist_filepath_string);

    // launchdで起動
    let launchctl = Command::new("launchctl")
        .arg("load")
        .arg(plist_filepath_string)
        .output()
        .expect("cannot run launchctl");
    let result_message = launchctl.stdout;
    println!("{}\n...\n", std::str::from_utf8(&result_message).unwrap());
}

pub fn get_complete_messages() -> String {
    return format!(
        "{}\n{}\n\nA plist file is saved as \"{}\"!\n",
        "Daemon setup is now completed!",
        "When you logs in a new launchd, PortSnippet process will be started by launchd.",
        PLIST_FILEPATH
    );
}
