use notify::{RecommendedWatcher, RecursiveMode, Watcher};

pub fn watch_dir(
    paths: Vec<String>,
    config: &super::Config,
    f: fn(&super::Config, &std::path::PathBuf),
) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;
    for path in paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    for res in rx {
        match res {
            Ok(event) => {
                if event.paths.len() > 0 && event.kind.is_modify() {
                    let code_filepath = &event.paths[0];
                    f(config, code_filepath);
                }
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
