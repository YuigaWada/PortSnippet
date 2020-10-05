use notify::{RecommendedWatcher, RecursiveMode, Watcher};

pub fn watch_dir(
    paths: Vec<String>,
    config: &super::Config,
    f: fn(&super::Config, notify::Event),
) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;
    for path in paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    for res in rx {
        match res {
            Ok(event) => f(config, event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
