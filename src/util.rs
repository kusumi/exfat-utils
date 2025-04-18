#[macro_export]
macro_rules! get_node {
    ($ef:expr, $nid:expr) => {
        $ef.get_node($nid).unwrap()
    };
}
pub use get_node;

#[macro_export]
macro_rules! get_node_mut {
    ($ef:expr, $nid:expr) => {
        $ef.get_node_mut($nid).unwrap()
    };
}
pub use get_node_mut;

pub fn print_version(prog: &str) {
    println!(
        "{} {}.{}.{}",
        match get_basename(prog) {
            Some(v) => v,
            None => "???".to_string(),
        },
        libexfat::VERSION[0],
        libexfat::VERSION[1],
        libexfat::VERSION[2]
    );
}

#[must_use]
pub fn get_basename(f: &str) -> Option<String> {
    Some(std::path::Path::new(&f).file_name()?.to_str()?.to_string())
}

const DEBUG: &str = "DEBUG";

#[must_use]
pub fn get_debug_level() -> i32 {
    match std::env::var(DEBUG) {
        Ok(v) => v.parse().unwrap_or(-1),
        Err(_) => -1,
    }
}

#[must_use]
pub fn is_debug_set() -> bool {
    get_debug_level() > 0
}

/// # Errors
pub fn init_std_logger() -> Result<(), log::SetLoggerError> {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", if is_debug_set() { "trace" } else { "info" });
    env_logger::try_init_from_env(env)
}
