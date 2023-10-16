use notify::Watcher;
use notify_debouncer_full::new_debouncer;
use std::time::Duration;

use crate::app::Command;

/// FullDebouncer is debouncert type returned by notify_debouncer_full crate when
/// we create a new debouncer
type FullDebouncer =
    notify_debouncer_full::Debouncer<notify::FsEventWatcher, notify_debouncer_full::FileIdMap>;
/// DebouncerReceiver is the std::sync::mpsc::Receiver type we send to the
/// notify_debouncer_full crate for receiving FS Change Events
type DebouncerReceiver = std::sync::mpsc::Receiver<
    Result<Vec<notify_debouncer_full::DebouncedEvent>, Vec<notify::Error>>,
>;

fn async_debouncer(debounce_secs: u64) -> notify::Result<(FullDebouncer, DebouncerReceiver)> {
    let (tx, rx) = std::sync::mpsc::channel();
    // TODO: this debouncer duration can be taken from bundle config as well
    // in key watcher:{ duration: Number(1) }
    let debouncer = new_debouncer(Duration::from_secs(debounce_secs), None, tx)?;
    Ok((debouncer, rx))
}

pub async fn async_watch<P: AsRef<std::path::Path>>(
    command: &Command,
    path: P,
    debounce_secs: u64,
) -> notify::Result<()> {
    println!("Starting to watch: {}", path.as_ref().to_string_lossy());
    let (mut debouncer, rx) = async_debouncer(debounce_secs)?;
    let mut child;
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer
        .watcher()
        .watch(path.as_ref(), notify::RecursiveMode::Recursive)?;
    // watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive)?;

    let mut cmd_proc = std::process::Command::new(&command.command);
    cmd_proc.args(&command.args);
    child = cmd_proc.spawn().unwrap();
    println!("starting first: {}", child.id());
    while let Ok(res) = rx.recv() {
        match res {
            Ok(event) => {
                println!("changed: {:?}", event);
                println!("killing: {}", child.id());
                child.kill()?;
                let mut cmd_proc = std::process::Command::new(&command.command);
                cmd_proc.args(&command.args);
                child = cmd_proc.spawn().unwrap();
                println!("started: {}", child.id());
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
