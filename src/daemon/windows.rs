use std::ffi::OsStr;
use std::process::Command;

const EXE_VARIABLE: &str = "{{EXE_PATH}}";
const REGISTER_ARGS: &'static [&'static str] = &[
    "create",
    "PortSnippet",
    "binPath=",
    r#"cmd /c "{{EXE_PATH}}" AUTO_LAUNCH"#,
    "DisplayName=",
    r#""PortSnippet""#,
    "start=",
    "auto",
];
const UNREGISTER_ARGS: &'static [&'static str] = &["delete", "PortSnippet"];
const START_ARGS: &'static [&'static str] = &["/c", "sc", "start", "PortSnippet"];

// Commandに引数渡す
fn take_args(command: &mut Command, args: Vec<&str>) {
    for arg in args {
        let arg_osstr = OsStr::new(arg);
        command.arg(arg_osstr);
    }
}

// EXE_VARIABLEを注入
fn inject_exe_variable(args: Vec<&str>, value: String) -> Vec<String> {
    return args
        .clone()
        .iter()
        .map(|s| s.replace(EXE_VARIABLE, &value))
        .collect::<Vec<String>>();
}

// プロセスを開始する
fn start_process() {
    let mut command = Command::new("cmd");
    take_args(&mut command, START_ARGS.to_vec());
    let _ = command.output();
}

// Serviceとして登録する
pub fn register(need_run: bool) {
    if need_run {
        unregister();
    }

    let exe_path = std::env::current_exe().expect("cannot get current_exe");
    let exe_path_string = exe_path
        .into_os_string()
        .into_string()
        .expect("cannot get current_exe");

    let register_args = inject_exe_variable(REGISTER_ARGS.to_vec(), exe_path_string);
    let register_args = register_args
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();

    let mut command = Command::new("sc");
    take_args(&mut command, register_args);
    let result = command.output();
    match result {
        Ok(result) => {
            let message = get_complete_messages();
            println!("{}", message);
        }
        Err(e) => {
            panic!("Error: {}", e);
        }
    }

    if need_run {
        start_process();
    }
}

// Serviceを解除する
fn unregister() {
    let mut command = Command::new("sc");
    take_args(&mut command, UNREGISTER_ARGS.to_vec());
    let result = command.output();
    match result {
        Ok(result) => {
            // let message =
            //     String::from_utf8(result.stdout).expect("cannot convert Vec<u8> to String.");
            // println!("Success: {}\n", message);
        }
        Err(e) => {
            panic!("Error: {}", e);
        }
    }
}

// Serviceとして登録した後にプロセスを起動する
pub fn run() {
    register(false);
    start_process();
}

// PortSnippetを停止する
pub fn stop() {
    unregister();
}

// 完了メッセージ
pub fn get_complete_messages() -> String {
    return [
        "Daemon setup is now completed!\n\n",
        "PortSnippet is registerd for the Windows service.\n",
        "PortSnnipet(service) will automatically start each time your computer is restarted.\n\n\n",
        "Please Exit ... (Ctrl + C)",
    ]
    .concat();
}
