mod dir;
mod xutil;

use exfat_utils::util;

use std::io::Read;

fn print_version() {
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

#[derive(Debug)]
struct ModfsParam {
    pc: dir::PathConflict,
}

impl ModfsParam {
    fn new(pc: dir::PathConflict) -> Self {
        Self { pc }
    }
}

fn write(
    ef: &mut libexfat::exfat::Exfat,
    nid: libexfat::node::Nid,
    p: &dir::Path,
) -> nix::Result<()> {
    let mut fp = match std::fs::File::open(p.get_src()) {
        Ok(v) => v,
        Err(e) => return Err(libexfat::util::error2errno(e)),
    };
    let mut buf = vec![0; 1 << 16];
    let mut offset = 0;
    loop {
        let bytes = match fp.read(&mut buf) {
            Ok(v) => v,
            Err(e) => return Err(libexfat::util::error2errno(e)),
        };
        if bytes == 0 {
            break;
        }
        let buf = &buf[..bytes];
        let bytes = ef.pwrite(nid, buf, offset)?;
        assert_eq!(bytes, buf.len().try_into().unwrap());
        offset += bytes;
    }
    ef.flush_node(nid)
}

fn modfs(spec: &str, input: &[String], param: &ModfsParam) -> nix::Result<()> {
    let mut v = vec![];
    for f in input {
        let mut x = match dir::collect(f) {
            Ok(v) => v,
            Err(e) => return Err(libexfat::util::error2errno(e)),
        };
        x.sort_by(|a, b| a.get_src().cmp(b.get_src()));
        v.extend_from_slice(&x);
    }
    let v = v;

    if let dir::PathConflict::Fail = param.pc {
        for i in 0..v.len() {
            for j in (i + 1)..v.len() {
                let p = &v[i];
                let q = &v[j];
                if (p.get_dst() == q.get_dst()) && !(p.is_dir() && q.is_dir()) {
                    log::error!("duplicate {} in {p:?} and {q:?}", p.get_dst());
                    return Err(nix::errno::Errno::EEXIST);
                }
            }
        }
    }

    let mut mopt = vec![];
    if util::is_debug_set() {
        mopt.push("--debug");
    }
    let mut ef = libexfat::mount(spec, &mopt)?;

    // fail before start writing
    if let dir::PathConflict::Fail = param.pc {
        for p in &v {
            let f = p.get_dst();
            assert!(f.starts_with('/'));
            if let Ok(v) = ef.lookup(f) {
                util::get_node_mut!(ef, v).put();
                if !(p.is_dir() && util::get_node!(ef, v).is_directory()) {
                    log::error!("{f} exists");
                    return Err(nix::errno::Errno::EEXIST);
                }
            }
        }
    }

    for p in &v {
        let f = p.get_dst();
        assert!(f.starts_with('/'));
        if let Ok(v) = ef.lookup(f) {
            log::warn!("{f} exists");
            match param.pc {
                dir::PathConflict::Fail => {
                    // entry with same name already exists (case insensitive)
                    // e.g. "Python" vs "python"
                    util::get_node_mut!(ef, v).put();
                    // mkdir / mknod will fail with EEXIST
                }
                dir::PathConflict::Ignore => {
                    util::get_node_mut!(ef, v).put();
                    continue;
                }
                dir::PathConflict::Unlink => {
                    if util::get_node!(ef, v).is_directory() {
                        util::get_node_mut!(ef, v).put();
                        continue;
                    }
                    ef.unlink(v)?;
                    log::info!("{f} unlinked");
                }
            }
        }
        if p.is_dir() {
            ef.mkdir(f)?;
        } else {
            let nid = ef.mknod(f)?;
            util::get_node_mut!(ef, nid).get();
            write(&mut ef, nid, p)?;
            util::get_node_mut!(ef, nid).put();
        }
    }
    Ok(())
}

fn usage(prog: &str, gopt: &getopts::Options) {
    print!(
        "{}",
        gopt.usage(&format!(
            "Usage: {prog} [-c \"fail\"|\"ignore\"|\"unlink\"] [-V] <device> <path> \
            [<extra-path>...]"
        ))
    );
}

fn main() {
    if let Err(e) = util::init_std_logger() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let args: Vec<String> = std::env::args().collect();
    let prog = &args[0];

    util::print_version(prog);

    let mut gopt = getopts::Options::new();
    gopt.optopt(
        "c",
        "conflict",
        "Action to take when a given path already exists within <device>. \
        \"fail\" fails with EEXIST unless both paths are directory. \
        \"ignore\" ignores a given path and leaves the existing path as is. \
        \"unlink\" unlinks the existing path first and then create. \
        Unlink of directory (and its child entries) is unsupported. \
        Defaults to \"fail\".",
        "<\"fail\"|\"ignore\"|\"unlink\">",
    );
    gopt.optflag("V", "version", "Print version and copyright.");
    gopt.optflag("h", "help", "Print usage.");

    let matches = match gopt.parse(&args[1..]) {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            usage(prog, &gopt);
            std::process::exit(1);
        }
    };
    if matches.opt_present("V") {
        print_version();
        std::process::exit(0);
    }
    if matches.opt_present("help") {
        usage(prog, &gopt);
        std::process::exit(0);
    }

    let pc = match matches.opt_str("c") {
        Some(v) => match v.to_lowercase().as_str() {
            "fail" => dir::PathConflict::Fail,
            "ignore" => dir::PathConflict::Ignore,
            "unlink" => dir::PathConflict::Unlink,
            _ => {
                log::error!("invalid option value: '{v}'");
                std::process::exit(1);
            }
        },
        None => dir::PathConflict::Fail,
    };

    let args = matches.free;
    if args.len() < 2 {
        usage(prog, &gopt);
        std::process::exit(1);
    }

    let param = ModfsParam::new(pc);
    log::debug!("param {param:?}");

    if let Err(e) = modfs(&args[0], &args[1..], &param) {
        log::error!("{e}");
        std::process::exit(1);
    }
}
