use exfat_utils::util;

use std::os::fd::AsRawFd;

fn print_version() {
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

fn usage(prog: &str, gopt: &getopts::Options) {
    let indent = "    ";
    print!(
        "{}",
        gopt.usage(&format!(
            "Usage: {prog} [-V] <\"nidprune\"> <path> \n\
            {indent}nidprune - Free in-memory nodes."
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

    let args = matches.free;
    if args.len() < 2 {
        usage(prog, &gopt);
        std::process::exit(1);
    }

    let cmd = &args[0];
    let f = &args[1];

    if cmd == "nidprune" {
        let fp = match std::fs::File::open(f) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        };
        nix::ioctl_read!(
            nidprune,
            libexfat::ctl::EXFAT_CTL,
            libexfat::ctl::EXFAT_CTL_NIDPRUNE,
            libexfat::ctl::ExfatCtlNidPruneData
        );
        let mut b = [0; 2];
        if let Err(e) = unsafe { nidprune(fp.as_raw_fd(), &mut b) } {
            log::error!("{e}"); // not supported if ENOTTY
            std::process::exit(1);
        }
        if cfg!(target_endian = "little") {
            b[0] = b[0].swap_bytes();
            b[1] = b[1].swap_bytes();
        }
        log::info!("{b:?}");
    } else {
        log::error!("invalid ioctl command {cmd}");
        std::process::exit(1);
    }
}
