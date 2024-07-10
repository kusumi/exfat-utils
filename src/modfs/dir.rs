use crate::util;

#[derive(Debug)]
pub(crate) enum PathConflict {
    Fail,
    Ignore,
    Unlink,
}

#[derive(Clone, Debug)]
pub(crate) struct Path {
    f: String,
    i: usize,
    is_dir: bool,
}

impl Path {
    pub(crate) fn new(f: String, i: usize, is_dir: bool) -> Self {
        Self { f, i, is_dir }
    }

    pub(crate) fn get_src(&self) -> &str {
        &self.f
    }

    pub(crate) fn get_dst(&self) -> &str {
        &self.f[self.i..]
    }

    pub(crate) fn is_dir(&self) -> bool {
        self.is_dir
    }
}

fn assert_path(f: &str) -> std::io::Result<()> {
    std::fs::metadata(f)?;
    assert!(util::is_abspath(f));
    assert!(!f.ends_with('/')); // must not end with /
    Ok(())
}

pub(crate) fn collect(input: &str) -> std::io::Result<Vec<Path>> {
    let input = util::canonicalize_path(input)?;
    assert_path(&input)?;

    let t = util::get_raw_file_type(&input)?;
    assert!(!t.is_symlink());
    let prefix = if t.is_dir() {
        input.clone()
    } else if t.is_file() {
        util::canonicalize_path(&util::get_dirpath(&input)?)?
    } else {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    };
    assert_path(&prefix)?;

    let t = util::get_raw_file_type(&prefix)?;
    if t.is_dir() || t.is_file() {
        walk_directory(&input, &prefix)
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::InvalidInput))
    }
}

fn walk_directory(input: &str, prefix: &str) -> std::io::Result<Vec<Path>> {
    let mut v = vec![];
    for entry in walkdir::WalkDir::new(input)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let Some(f) = entry.path().to_str() else {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        };
        assert_path(f)?;
        if f == prefix {
            continue;
        }
        let t = util::get_raw_file_type(f)?;
        if t.is_dir() || t.is_file() {
            v.push(Path::new(f.to_string(), prefix.len(), t.is_dir()));
        } else {
            log::warn!("ignore unsupported file: {f} ({t:?})");
            continue;
        }
    }
    Ok(v)
}
