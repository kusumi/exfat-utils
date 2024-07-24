#[macro_export]
macro_rules! get_node {
    ($ef:expr, $nid:expr) => {
        $ef.get_node($nid).unwrap()
    };
}
pub use get_node;

#[macro_export]
macro_rules! get_mut_node {
    ($ef:expr, $nid:expr) => {
        $ef.get_mut_node($nid).unwrap()
    };
}
pub use get_mut_node;

pub fn print_version(prog: &str) {
    println!(
        "{} {}.{}.{}",
        get_basename(prog),
        libexfat::VERSION[0],
        libexfat::VERSION[1],
        libexfat::VERSION[2]
    );
}

#[must_use]
pub fn get_basename(f: &str) -> String {
    std::path::Path::new(&f)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

const DEBUG: &str = "DEBUG";

#[must_use]
pub fn get_debug_level() -> i32 {
    match std::env::var(DEBUG) {
        Ok(v) => v.parse::<i32>().unwrap_or(-1),
        Err(_) => -1,
    }
}

#[must_use]
pub fn is_debug_set() -> bool {
    get_debug_level() > 0
}

pub fn init_std_logger() -> Result<(), log::SetLoggerError> {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", if is_debug_set() { "trace" } else { "info" });
    env_logger::try_init_from_env(env)
}
