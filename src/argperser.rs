#[derive(Debug, PartialEq)]
pub enum LaunchType {
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

// パラメータ(引数)からLaunchTypeを特定する

#[cfg(debug_assertions)] // デバッグ用
pub fn detect_type(_args: Vec<String>) -> LaunchType {
    return LaunchType::Daemon;
}

#[cfg(not(debug_assertions))] // リリース用
pub fn detect_type(args: Vec<String>) -> LaunchType {
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

// パラメータ一覧を取得
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

// ヘルプを表示
pub fn print_help() {}

#[cfg(not(debug_assertions))]
mod tests {
    use crate::argperser::*;

    // util

    fn gen_mock_args(arg: &str) -> Vec<String> {
        return vec!["MOCK_EXE_PATH".to_string(), arg.to_string()];
    }

    // shorts

    #[test]
    fn daemon_valid() {
        let launch_type = detect_type(gen_mock_args("AUTO_LAUNCH"));
        assert_eq!(launch_type, LaunchType::Daemon);
    }

    #[test]
    fn short_daemon_invalid() {
        let launch_type = detect_type(gen_mock_args("A"));
        assert_ne!(launch_type, LaunchType::Daemon); // not!
    }

    #[test]
    fn short_man_valid() {
        let launch_type = detect_type(gen_mock_args("-m"));
        assert_eq!(launch_type, LaunchType::Daemon);
    }

    #[test]
    fn short_stop_valid() {
        let launch_type = detect_type(gen_mock_args("-s"));
        assert_eq!(launch_type, LaunchType::Stop);
    }

    #[test]
    fn short_restart_valid() {
        let launch_type = detect_type(gen_mock_args("-r"));
        assert_eq!(launch_type, LaunchType::Restart);
    }

    #[test]
    fn short_help_valid() {
        let launch_type = detect_type(gen_mock_args("-h"));
        assert_eq!(launch_type, LaunchType::Help);
    }

    // long

    #[test]
    fn long_man_valid() {
        let launch_type = detect_type(gen_mock_args("man"));
        assert_eq!(launch_type, LaunchType::Daemon);
    }

    #[test]
    fn long_stop_valid() {
        let launch_type = detect_type(gen_mock_args("stop"));
        assert_eq!(launch_type, LaunchType::Stop);
    }

    #[test]
    fn long_restart_valid() {
        let launch_type = detect_type(gen_mock_args("restart"));
        assert_eq!(launch_type, LaunchType::Restart);
    }

    #[test]
    fn long_help_valid() {
        let launch_type = detect_type(gen_mock_args("help"));
        assert_eq!(launch_type, LaunchType::Help);
    }
}
