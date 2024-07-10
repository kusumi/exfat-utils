fn print_version(prog: &str) {
    exfat_utils::util::print_version(prog);
    println!("Copyright (C) 2011-2023  Andrew Nayenko");
    println!("Copyright (C) 2020-2023  Endless OS Foundation LLC");
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

fn print_attribute(attribs: u16, attrib: u16, label: &str) {
    println!(
        "{label:>9}: {}",
        if (attribs & attrib) != 0 { "yes" } else { "no" }
    );
}

fn attribute(
    ef: &mut libexfat::exfat::Exfat,
    nid: libexfat::node::Nid,
    add_flags: u16,
    clear_flags: u16,
) -> nix::Result<()> {
    if (add_flags | clear_flags) != 0 {
        let node = exfat_utils::get_mut_node!(ef, nid);
        let mut attrib = node.get_attrib();
        attrib |= add_flags;
        attrib &= !clear_flags;
        if node.get_attrib() != attrib {
            node.set_attrib(attrib);
            node.set_is_dirty();
            if let Err(e) = ef.flush_node(nid) {
                log::error!(
                    "failed to flush changes to {}: {e}",
                    exfat_utils::get_node!(ef, nid).get_name()
                );
                return Err(e);
            }
        }
    } else {
        let attrib = exfat_utils::get_node!(ef, nid).get_attrib();
        print_attribute(attrib, libexfat::exfatfs::EXFAT_ATTRIB_RO, "Read-only");
        print_attribute(attrib, libexfat::exfatfs::EXFAT_ATTRIB_HIDDEN, "Hidden");
        print_attribute(attrib, libexfat::exfatfs::EXFAT_ATTRIB_SYSTEM, "System");
        print_attribute(attrib, libexfat::exfatfs::EXFAT_ATTRIB_ARCH, "Archive");
        // read-only attributes
        print_attribute(attrib, libexfat::exfatfs::EXFAT_ATTRIB_VOLUME, "Volume");
        print_attribute(attrib, libexfat::exfatfs::EXFAT_ATTRIB_DIR, "Directory");
    }
    Ok(())
}

fn usage(prog: &str, opts: &getopts::Options) {
    print!(
        "{}",
        opts.usage(&format!(
            "Usage: {prog} -d <device> <file>\n       {prog} [FLAGS] -d <device> <file>"
        ))
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = &args[0];

    let mut opts = getopts::Options::new();
    opts.optopt(
        "d",
        "",
        "The path to an unmounted disk partition or disk image file containing \
        an exFAT file system. \
        This option is required.",
        "<device>",
    );
    opts.optflag("r", "", "Set read-only flag");
    opts.optflag("R", "", "Clear read-only flag");
    opts.optflag("i", "", "Set hidden flag (mnemonic: invisible)");
    opts.optflag("I", "", "Clear hidden flag");
    opts.optflag("s", "", "Set system flag");
    opts.optflag("S", "", "Clear system flag");
    opts.optflag("a", "", "Set archive flag");
    opts.optflag("A", "", "Clear archive flag");
    opts.optflag("V", "version", "Print version and copyright.");
    opts.optflag("h", "help", "Print usage.");
    opts.optflag("", "debug", "");

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

    // The path to the unmounted exFAT partition is a (mandatory) named
    // option rather than a positional parameter. If the FUSE file system
    // ever gains an ioctl to get and set attributes, this option could be
    // made optional, and this tool taught to use the ioctl.
    let spec = matches.opt_str("d");

    let mut add_flags = 0;
    let mut clear_flags = 0;
    if matches.opt_present("r") {
        add_flags |= libexfat::exfatfs::EXFAT_ATTRIB_RO;
    }
    if matches.opt_present("R") {
        clear_flags |= libexfat::exfatfs::EXFAT_ATTRIB_RO;
    }
    // "-h[elp]" is taken; i is the second letter of "hidden" and
    // its synonym "invisible"
    if matches.opt_present("i") {
        add_flags |= libexfat::exfatfs::EXFAT_ATTRIB_HIDDEN;
    }
    if matches.opt_present("I") {
        clear_flags |= libexfat::exfatfs::EXFAT_ATTRIB_HIDDEN;
    }
    if matches.opt_present("s") {
        add_flags |= libexfat::exfatfs::EXFAT_ATTRIB_SYSTEM;
    }
    if matches.opt_present("S") {
        clear_flags |= libexfat::exfatfs::EXFAT_ATTRIB_SYSTEM;
    }
    if matches.opt_present("a") {
        add_flags |= libexfat::exfatfs::EXFAT_ATTRIB_ARCH;
    }
    if matches.opt_present("A") {
        clear_flags |= libexfat::exfatfs::EXFAT_ATTRIB_ARCH;
    }

    let debug = matches.opt_present("debug");

    if let Err(e) = exfat_utils::util::init_std_logger(debug) {
        log::error!("{e}");
        std::process::exit(1);
    }

    let mut mopts = vec![];
    if debug {
        mopts.push("--debug");
    }

    if (add_flags & clear_flags) != 0 {
        log::error!("can't set and clear the same flag");
        std::process::exit(1);
    }
    if (add_flags | clear_flags) == 0 {
        mopts.extend_from_slice(&["--mode", "ro"]);
    }

    let args = matches.free;
    if spec.is_none() || args.len() != 1 {
        usage(prog, &opts);
        std::process::exit(1);
    }
    let spec = spec.unwrap();

    let mut ef = match libexfat::mount(&spec, &mopts) {
        Ok(v) => v,
        Err(e) => {
            log::error!("failed to mount {spec}: {e}");
            std::process::exit(1);
        }
    };

    let file_path = &args[0];
    let nid = match ef.lookup(file_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("failed to look up '{file_path}': {e}");
            std::process::exit(1);
        }
    };

    let result = attribute(&mut ef, nid, add_flags, clear_flags);
    exfat_utils::get_mut_node!(ef, nid).put();
    if let Err(e) = result {
        log::error!("{e}");
        std::process::exit(1);
    }
}
