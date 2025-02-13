use path_clean::PathClean;

pub(crate) fn canonicalize_path(f: &str) -> exfat_utils::Result<String> {
    Ok(std::fs::canonicalize(f)?
        .into_os_string()
        .into_string()
        .unwrap())
}

// This function
// * does not resolve symlink
// * works with non existent path
pub(crate) fn get_abspath(f: &str) -> exfat_utils::Result<String> {
    let p = std::path::Path::new(f);
    Ok(if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(f)
    }
    .clean()
    .into_os_string()
    .into_string()
    .unwrap())
}

// fails if f is "/" or equivalent
pub(crate) fn get_dirpath(f: &str) -> exfat_utils::Result<String> {
    Ok(std::path::Path::new(&get_abspath(f)?)
        .parent()
        .ok_or(nix::errno::Errno::ENOENT)?
        .to_str()
        .ok_or(nix::errno::Errno::ENOENT)?
        .to_string())
}

pub(crate) fn is_abspath(f: &str) -> bool {
    std::path::Path::new(f).is_absolute()
}

pub(crate) fn get_raw_file_type(f: &str) -> std::io::Result<std::fs::FileType> {
    Ok(std::fs::symlink_metadata(f)?.file_type())
}
