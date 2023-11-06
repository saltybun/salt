mod app;
mod watcher;

use app::interface::Interface;

// Notes:
// pass commonly used settings to devs: SALT_ENV , SALT_ARCH , SALT_OS , SALT_ARGS , SALT_PWD
// .salt will be the cache directory

fn main() -> std::io::Result<()> {
    let mut app = Interface::init()?;
    let args: Vec<String> = std::env::args().collect();
    match app.run(&args) {
        Ok(_) => app.save_to_history(&args),
        Err(e) => Err(e),
    }
}
