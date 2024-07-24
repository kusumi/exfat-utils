fn print_version(prog: &str) {
    exfat_utils::util::print_version(prog);
    println!("Copyright (C) 2011-2023  Andrew Nayenko");
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

fn usage(prog: &str, opts: &getopts::Options) {
    print!(
        "{}",
        opts.usage(&format!("Usage: {prog} [-V] <device> [label]"))
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

    let mut mopts = vec![];
    if exfat_utils::util::is_debug_set() {
        mopts.push("--debug");
    }

    let args = matches.free;
    if args.len() != 1 && args.len() != 2 {
        usage(prog, &opts);
        std::process::exit(1);
    }
    let spec = &args[0];

    if args.len() == 1 {
        mopts.extend_from_slice(&["--mode", "ro"]);
        let ef = match libexfat::mount(spec, &mopts) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        };
        println!("{}", ef.get_label());
    } else if args.len() == 2 {
        let mut ef = match libexfat::mount(spec, &mopts) {
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
        usage(prog, &opts);
        std::process::exit(1);
    }
}
