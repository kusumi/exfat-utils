use std::io::Write;

fn print_version() {
    println!("Copyright (C) 2011-2023  Andrew Nayenko");
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

fn print_super_block(ef: &libexfat::exfat::Exfat) {
    let sb = ef.get_super_block();
    let total_space = u64::from_le(sb.sector_count) * sb.get_sector_size();
    let avail_space = u64::from(ef.get_free_clusters().unwrap()) * sb.get_cluster_size();
    println!(
        "File system version           {}.{}",
        sb.version_major, sb.version_minor
    );
    let (value, unit) = libexfat::util::humanize_bytes(sb.get_sector_size());
    println!("Sector size          {value:>10} {unit}");
    let (value, unit) = libexfat::util::humanize_bytes(sb.get_cluster_size());
    println!("Cluster size         {value:>10} {unit}");
    let (value, unit) = libexfat::util::humanize_bytes(total_space);
    println!("Volume size          {value:>10} {unit}");
    let (value, unit) = libexfat::util::humanize_bytes(total_space - avail_space);
    println!("Used space           {value:>10} {unit}");
    let (value, unit) = libexfat::util::humanize_bytes(avail_space);
    println!("Available space      {value:>10} {unit}");
}

fn nodeck(ef: &mut libexfat::exfat::Exfat, nid: libexfat::node::Nid) -> exfat_utils::Result<()> {
    let cluster_size = ef.get_cluster_size();
    let node = exfat_utils::util::get_node!(ef, nid);
    let mut clusters = libexfat::div_round_up!(node.get_size(), cluster_size);
    let mut c = node.get_start_cluster();

    while clusters > 0 {
        clusters -= 1;
        if ef.cluster_invalid(c) {
            log::error!(
                "file '{}' has invalid cluster {c:#x}",
                exfat_utils::util::get_node!(ef, nid).get_name()
            );
            return Err(Box::new(nix::errno::Errno::EINVAL));
        }
        if !ef.is_cluster_allocated((c - libexfat::fs::EXFAT_FIRST_DATA_CLUSTER).try_into()?)? {
            log::error!(
                "cluster {c:#x} of file '{}' is not allocated",
                exfat_utils::util::get_node!(ef, nid).get_name()
            );
            return Err(Box::new(nix::errno::Errno::EINVAL));
        }
        c = ef.next_cluster(nid, c);
    }
    Ok(())
}

fn dirck(ef: &mut libexfat::exfat::Exfat, path: &str) -> exfat_utils::Result<(u64, u64)> {
    let dnid = match ef.lookup(path) {
        Ok(v) => v,
        Err(e) => panic!("directory '{path}' is not found: {e}"),
    };
    let dnode = exfat_utils::util::get_node!(ef, dnid);
    assert!(
        dnode.is_directory(),
        "'{path}' is not a directory ({:#x})",
        dnode.get_attrib()
    );
    if let Err(e) = nodeck(ef, dnid) {
        exfat_utils::util::get_node_mut!(ef, dnid).put();
        return Err(e);
    }

    let mut c = match ef.opendir_cursor(dnid) {
        Ok(v) => v,
        Err(e) => {
            exfat_utils::util::get_node_mut!(ef, dnid).put();
            return Err(Box::new(e));
        }
    };

    let mut directories_count = 0;
    let mut files_count = 0;
    loop {
        let nid = match ef.readdir_cursor(&mut c) {
            Ok(v) => v,
            Err(e) => {
                if let libexfat::Error::Errno(e) = e {
                    if e == nix::errno::Errno::ENOENT {
                        break;
                    }
                }
                ef.closedir_cursor(c);
                exfat_utils::util::get_node_mut!(ef, dnid).put();
                return Err(Box::new(e));
            }
        };
        let node = exfat_utils::util::get_node!(ef, nid);
        let entry_path = format!("{}/{}", path, node.get_name());
        log::debug!(
            "{}: {}, {} bytes, cluster {}",
            entry_path,
            if node.get_is_contiguous() {
                "contiguous"
            } else {
                "fragmented"
            },
            node.get_size(),
            node.get_start_cluster()
        );
        if node.is_directory() {
            directories_count += 1;
            let (d, f) = match dirck(ef, &entry_path) {
                Ok(v) => v,
                Err(e) => {
                    exfat_utils::util::get_node_mut!(ef, nid).put();
                    ef.closedir_cursor(c);
                    exfat_utils::util::get_node_mut!(ef, dnid).put();
                    return Err(e);
                }
            };
            directories_count += d;
            files_count += f;
        } else {
            files_count += 1;
            if let Err(e) = nodeck(ef, nid) {
                log::error!("{e}");
            }
        }
        if let Err(e) = ef.flush_node(nid) {
            exfat_utils::util::get_node_mut!(ef, nid).put();
            ef.closedir_cursor(c);
            exfat_utils::util::get_node_mut!(ef, dnid).put();
            return Err(Box::new(e));
        }
        exfat_utils::util::get_node_mut!(ef, nid).put();
    }

    ef.closedir_cursor(c);
    if let Err(e) = ef.flush_node(dnid) {
        exfat_utils::util::get_node_mut!(ef, dnid).put();
        return Err(Box::new(e));
    }
    exfat_utils::util::get_node_mut!(ef, dnid).put();

    Ok((directories_count, files_count))
}

fn fsck(spec: &str, mopt: &[&str]) -> exfat_utils::Result<Option<libexfat::exfat::Exfat>> {
    // ENODEV - failed to open the device, checking haven't started
    let mut ef = match libexfat::mount(spec, mopt) {
        Ok(v) => v,
        Err(e) => {
            if let libexfat::Error::Errno(e) = e {
                if e == nix::errno::Errno::ENODEV {
                    return Err(Box::new(nix::errno::Errno::ENODEV));
                }
            }
            log::error!("{e}");
            print!("File system checking stopped. ");
            std::io::stdout().flush()?;
            return Ok(None);
        }
    };

    print_super_block(&ef);
    ef.soil_super_block()?;
    let (directories_count, files_count) = dirck(&mut ef, "")?;

    println!("Totally {directories_count} directories and {files_count} files.");
    print!("File system checking finished. ");
    std::io::stdout().flush()?;
    Ok(Some(ef))
}

fn usage(prog: &str, gopt: &getopts::Options) {
    print!(
        "{}",
        gopt.usage(&format!("Usage: {prog} [-a | -n | -p | -y] [-V] <device>"))
    );
}

fn main() {
    if let Err(e) = exfat_utils::util::init_std_logger() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let args: Vec<String> = std::env::args().collect();
    let prog = &args[0];

    exfat_utils::util::print_version(prog);

    let mut gopt = getopts::Options::new();
    gopt.optflag(
        "a",
        "",
        "Automatically repair the file system. No user intervention required.",
    );
    gopt.optflag(
        "n",
        "",
        "No-operation mode: non-interactively check for errors, \
        but don't write anything to the file system.",
    );
    gopt.optflag("p", "", "Same as -a for compatibility with other *fsck.");
    gopt.optflag("y", "", "Same as -a for compatibility with other *fsck.");
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

    let mut mopt = vec![];
    if libfs::is_debug_set() {
        mopt.push("--debug");
    }

    if matches.opt_present("a") || matches.opt_present("p") || matches.opt_present("y") {
        mopt.extend_from_slice(&["--repair", "yes"]);
    } else if matches.opt_present("n") {
        mopt.extend_from_slice(&["--repair", "no", "--mode", "ro"]);
    } else {
        let repair = match nix::unistd::isatty(0) {
            Ok(v) => {
                if v {
                    "ask"
                } else {
                    "no"
                }
            }
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        };
        mopt.extend_from_slice(&["--repair", repair]);
    }

    let args = matches.free;
    if args.len() != 1 {
        usage(prog, &gopt);
        std::process::exit(1);
    }
    let spec = &args[0];

    println!("Checking file system on {spec}.");
    let ef = match fsck(spec, &mopt) {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };
    if let Some(ef) = ef {
        if ef.get_errors() != 0 {
            log::error!(
                "ERRORS FOUND: {}, FIXED: {}.",
                ef.get_errors(),
                ef.get_errors_fixed()
            );
            std::process::exit(1);
        }
    }
    println!("No errors found.");
}
