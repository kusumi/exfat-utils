fn print_version(prog: &str) {
    exfat_utils::util::print_version(prog);
    println!("Copyright (C) 2011-2023  Andrew Nayenko");
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

fn print_generic_info(sb: &libexfat::exfatfs::ExfatSuperBlock) {
    println!(
        "Volume serial number      0x{:08x}",
        u32::from_le(sb.volume_serial)
    );
    println!(
        "FS version                       {}.{}",
        sb.version_major, sb.version_minor
    );
    println!("Sector size               {:>10}", sb.get_sector_size());
    println!("Cluster size              {:>10}", sb.get_cluster_size());
}

fn print_sector_info(sb: &libexfat::exfatfs::ExfatSuperBlock) {
    println!(
        "Sectors count             {:>10}",
        u64::from_le(sb.sector_count)
    );
}

fn print_cluster_info(sb: &libexfat::exfatfs::ExfatSuperBlock) {
    println!(
        "Clusters count            {:>10}",
        u32::from_le(sb.cluster_count)
    );
}

fn print_other_info(sb: &libexfat::exfatfs::ExfatSuperBlock) {
    println!(
        "First sector              {:>10}",
        u64::from_le(sb.sector_start)
    );
    println!(
        "FAT first sector          {:>10}",
        u32::from_le(sb.fat_sector_start)
    );
    println!(
        "FAT sectors count         {:>10}",
        u32::from_le(sb.fat_sector_count)
    );
    println!(
        "First cluster sector      {:>10}",
        u32::from_le(sb.cluster_sector_start)
    );
    println!(
        "Root directory cluster    {:>10}",
        u32::from_le(sb.rootdir_cluster)
    );
    println!(
        "Volume state                  0x{:04x}",
        u16::from_le(sb.volume_state)
    );
    println!("FATs count                {:>10}", sb.fat_count);
    println!("Drive number                    0x{:02x}", sb.drive_no);
    println!("Allocated space           {:>9}%", sb.allocated_percent);
}

fn dump_sb(spec: &str) -> std::io::Result<()> {
    let mut dev = libexfat::open(spec, "ro")?;
    let buf = match dev.readx(libexfat::exfatfs::EXFAT_SUPER_BLOCK_SIZE_U64) {
        Ok(v) => v,
        Err(e) => {
            log::error!("failed to read from '{spec}'");
            return Err(e);
        }
    };
    let (prefix, body, suffix) = unsafe { buf.align_to::<libexfat::exfatfs::ExfatSuperBlock>() };
    assert!(prefix.is_empty());
    assert!(suffix.is_empty());
    let sb = body[0];

    if sb.oem_name != "EXFAT   ".as_bytes() {
        log::error!("exFAT file system is not found on '{spec}'");
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }

    print_generic_info(&sb);
    print_sector_info(&sb);
    print_cluster_info(&sb);
    print_other_info(&sb);
    Ok(())
}

fn dump_full(spec: &str, used_sectors: bool) -> nix::Result<()> {
    let mut mopts = vec!["--mode", "ro"];
    if exfat_utils::util::is_debug_set() {
        mopts.push("--debug");
    }

    let ef = libexfat::mount(spec, &mopts)?;
    let free_clusters = ef.get_free_clusters();
    let sb = ef.get_super_block();
    let free_sectors = free_clusters << sb.spc_bits;

    println!("Volume label         {:>15}", ef.get_label());
    print_generic_info(&sb);
    print_sector_info(&sb);
    println!("Free sectors              {free_sectors:>10}");
    print_cluster_info(&sb);
    println!("Free clusters             {free_clusters:>10}");
    print_other_info(&sb);

    if used_sectors {
        let mut a = 0;
        let mut b = 0;
        print!("Used sectors ");
        while ef.find_used_sectors(&mut a, &mut b)? {
            print!(" {a}-{b}");
        }
        println!();
    }
    Ok(())
}

fn dump_file_fragments(spec: &str, path: &str) -> nix::Result<()> {
    let mut mopts = vec!["--mode", "ro"];
    if exfat_utils::util::is_debug_set() {
        mopts.push("--debug");
    }

    let mut ef = libexfat::mount(spec, &mopts)?;
    let nid = match ef.lookup(path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("'{path}': {e}");
            return Err(e);
        }
    };

    let node = exfat_utils::get_node!(ef, nid);
    let mut cluster = node.get_start_cluster();
    let mut fragment_start_cluster = cluster;
    let mut remainder = node.get_size();
    let mut fragment_size = 0;

    while remainder > 0 {
        if ef.cluster_invalid(cluster) {
            exfat_utils::get_mut_node!(ef, nid).put();
            log::error!("'{path}' has invalid cluster {cluster:#x}");
            return Err(nix::errno::Errno::EIO);
        }
        let lsize = std::cmp::min(ef.get_cluster_size(), remainder);
        fragment_size += lsize;
        remainder -= lsize;

        let next_cluster = ef.next_cluster(nid, cluster);
        if next_cluster != cluster + 1 || remainder == 0 {
            // next cluster is not contiguous or this is EOF
            println!("{} {}", ef.c2o(fragment_start_cluster), fragment_size);
            // start a new fragment
            fragment_start_cluster = next_cluster;
            fragment_size = 0;
        }
        cluster = next_cluster;
    }

    exfat_utils::get_mut_node!(ef, nid).put();
    Ok(())
}

fn usage(prog: &str, opts: &getopts::Options) {
    print!(
        "{}",
        opts.usage(&format!("Usage: {prog} [-s] [-u] [-f file] [-V] <device>"))
    );
}

fn main() {
    if let Err(e) = exfat_utils::util::init_std_logger() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let args: Vec<String> = std::env::args().collect();
    let prog = &args[0];

    let mut opts = getopts::Options::new();
    opts.optflag(
        "s",
        "",
        "Dump only info from super block. May be useful for heavily corrupted file systems.",
    );
    opts.optflag(
        "u",
        "",
        "Dump ranges of used sectors starting from 0 and separated with spaces. \
        May be useful for backup tools.",
    );
    opts.optopt(
        "f",
        "",
        "Print out a list of fragments that compose the given file. \
        Each fragment is printed on its own line, as the start offset (in bytes) \
        into the file system, and the length (in bytes).",
        "<file>",
    );
    opts.optflag("V", "version", "Print version and copyright.");
    opts.optflag("h", "help", "Print usage.");

    let matches = match opts.parse(&args[1..]) {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            usage(prog, &opts);
            std::process::exit(1);
        }
    };
    if matches.opt_present("V") {
        print_version(prog);
        std::process::exit(0);
    }
    if matches.opt_present("help") {
        usage(prog, &opts);
        std::process::exit(0);
    }

    let sb_only = matches.opt_present("s");
    let used_sectors = matches.opt_present("u");
    let file_path = matches.opt_str("f");

    let args = matches.free;
    if args.len() != 1 {
        usage(prog, &opts);
        std::process::exit(1);
    }
    let spec = &args[0];

    if let Some(file_path) = file_path {
        if let Err(e) = dump_file_fragments(spec, &file_path) {
            log::error!("{e}");
            std::process::exit(1);
        }
    } else if sb_only {
        if let Err(e) = dump_sb(spec) {
            log::error!("{e}");
            std::process::exit(1);
        }
    } else if true {
        if let Err(e) = dump_full(spec, used_sectors) {
            log::error!("{e}");
            std::process::exit(1);
        }
    } else {
        unreachable!();
    }
}
