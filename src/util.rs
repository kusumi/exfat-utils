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
        match libfs::fs::get_base_name(prog) {
            Some(v) => v,
            None => "???".to_string(),
        },
        libexfat::VERSION[0],
        libexfat::VERSION[1],
        libexfat::VERSION[2]
    );
}

/// # Errors
pub fn init_std_logger() -> Result<(), log::SetLoggerError> {
    let env = env_logger::Env::default().filter_or(
        "RUST_LOG",
        if libfs::is_debug_set() {
            "trace"
        } else {
            "info"
        },
    );
    env_logger::try_init_from_env(env)
}
