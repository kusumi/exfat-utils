fn print_version(prog: &str) {
    exfat_utils::util::print_version(prog);
    println!("Copyright (C) 2011-2023  Andrew Nayenko");
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

fn usage(prog: &str, gopt: &getopts::Options) {
    print!(
        "{}",
        gopt.usage(&format!("Usage: {prog} [-V] <device> [label]"))
    );
}

fn main() {
    if let Err(e) = exfat_utils::util::init_std_logger() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let args: Vec<String> = std::env::args().collect();
    let prog = &args[0];

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
        print_version(prog);
        std::process::exit(0);
    }
    if matches.opt_present("help") {
        usage(prog, &gopt);
        std::process::exit(0);
    }

    let mut mopt = vec![];
    if exfat_utils::util::is_debug_set() {
        mopt.push("--debug");
    }

    let args = matches.free;
    if args.len() != 1 && args.len() != 2 {
        usage(prog, &gopt);
        std::process::exit(1);
    }
    let spec = &args[0];

    if args.len() == 1 {
        mopt.extend_from_slice(&["--mode", "ro"]);
        let ef = match libexfat::mount(spec, &mopt) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        };
        println!("{}", ef.get_label());
    } else if args.len() == 2 {
        let mut ef = match libexfat::mount(spec, &mopt) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        };
        if let Err(e) = ef.set_label(&args[1]) {
            log::error!("{e}");
            std::process::exit(1);
        }
    } else {
        usage(prog, &gopt);
        std::process::exit(1);
    }
}
