mod cbm;
mod fat;
mod mkexfat;
mod rootdir;
mod uct;
mod uctc;
mod vbr;

const CHAR_BIT: usize = 8;

fn print_version() {
    println!("Copyright (C) 2011-2023  Andrew Nayenko");
    println!("Copyright (C) 2024-  Tomohiro Kusumi");
}

#[derive(Clone, Copy, Debug)]
struct MkfsParam {
    sector_bits: i32,
    spc_bits: i32,
    volume_size: u64,
    volume_label: [u16; libexfat::exfatfs::EXFAT_ENAME_MAX],
    volume_serial: u32,
    first_sector: u64,
    sector_size: u64,
    cluster_size: u64,
}

impl MkfsParam {
    fn new(
        sector_bits: i32,
        spc_bits: i32,
        volume_size: u64,
        volume_label: [u16; libexfat::exfatfs::EXFAT_ENAME_MAX],
        volume_serial: u32,
        first_sector: u64,
    ) -> Self {
        let sector_size = 1 << sector_bits;
        let cluster_size = sector_size << spc_bits;
        Self {
            sector_bits,
            spc_bits,
            volume_size,
            volume_label,
            volume_serial,
            first_sector,
            sector_size,
            cluster_size,
        }
    }
}

fn setup_spc_bits(sector_bits: i32, user_defined: i32, volume_size: u64) -> nix::Result<i32> {
    if user_defined != -1 {
        let cluster_size = (1 << sector_bits) << user_defined;
        if volume_size / cluster_size > libexfat::exfatfs::EXFAT_LAST_DATA_CLUSTER.into() {
            let (chb_value, chb_unit) = libexfat::util::humanize_bytes(cluster_size);
            let (vhb_value, vhb_unit) = libexfat::util::humanize_bytes(volume_size);
            log::error!(
                "cluster size {chb_value} {chb_unit} is too small for \
                {vhb_value} {vhb_unit} volume, try -s {}",
                1 << setup_spc_bits(sector_bits, -1, volume_size).unwrap()
            );
            return Err(nix::errno::Errno::EINVAL);
        }
        return Ok(user_defined);
    }
    if volume_size < 256 * 1024 * 1024 {
        return Ok(std::cmp::max(0, 12 - sector_bits)); // 4 KB
    }
    if volume_size < 32 * 1024 * 1024 * 1024 {
        return Ok(std::cmp::max(0, 15 - sector_bits)); // 32 KB
    }
    let mut i = 17; // 128 KB or more
    loop {
        if libexfat::util::div_round_up!(volume_size, 1 << i)
            <= libexfat::exfatfs::EXFAT_LAST_DATA_CLUSTER.into()
        {
            return Ok(std::cmp::max(0, i - sector_bits));
        }
        i += 1;
    }
}

fn setup_volume_label(s: &str) -> nix::Result<[u16; libexfat::exfatfs::EXFAT_ENAME_MAX]> {
    if s.is_empty() {
        return Ok([0; libexfat::exfatfs::EXFAT_ENAME_MAX]);
    }
    let s = s.as_bytes();
    Ok(
        libexfat::utf::utf8_to_utf16(s, libexfat::exfatfs::EXFAT_ENAME_MAX, s.len())?
            .try_into()
            .unwrap(),
    )
}

fn setup_volume_serial(user_defined: u32) -> nix::Result<u32> {
    if user_defined != 0 {
        return Ok(user_defined);
    }
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(v) => Ok(((v.as_secs() as u32) << 20) | v.subsec_micros()),
        Err(_) => Err(nix::errno::Errno::EINVAL),
    }
}

fn setup(
    dev: &mut libexfat::device::ExfatDevice,
    sector_bits: i32,
    spc_bits: i32,
    volume_label: &str,
    volume_serial: u32,
    first_sector: u64,
) -> nix::Result<()> {
    let volume_size = dev.get_size();
    let spc_bits = match setup_spc_bits(sector_bits, spc_bits, volume_size) {
        Ok(v) => v,
        Err(e) => {
            log::error!("invalid spc_bits {spc_bits}");
            return Err(e);
        }
    };
    let volume_label = match setup_volume_label(volume_label) {
        Ok(v) => v,
        Err(e) => {
            log::error!("invalid volume_label '{volume_label}'");
            return Err(e);
        }
    };
    let volume_serial = match setup_volume_serial(volume_serial) {
        Ok(v) => v,
        Err(e) => {
            log::error!("invalid volume_serial '{volume_serial}'");
            return Err(e);
        }
    };
    let param = MkfsParam::new(
        sector_bits,
        spc_bits,
        volume_size,
        volume_label,
        volume_serial,
        first_sector,
    );
    match mkexfat::mkfs(dev, &param) {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!("{e}");
            Err(libexfat::util::error2errno(e))
        }
    }
}

fn logarithm2(n: i32) -> i32 {
    let bits = std::mem::size_of::<i32>() * CHAR_BIT - 1;
    for i in 0..bits {
        if (1 << i) == n {
            return i.try_into().unwrap();
        }
    }
    -1
}

fn usage(prog: &str, opts: &getopts::Options) {
    print!(
        "{}",
        opts.usage(&format!(
            "Usage: {prog} [-i volume-id] [-n label] [-p partition-first-sector] \
            [-s sectors-per-cluster] [-V] <device>"
        ))
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = &args[0];

    exfat_utils::util::print_version(prog);

    let mut opts = getopts::Options::new();
    opts.optopt(
        "i",
        "",
        "A 32-bit hexadecimal number. By default a value based on current time is set. \
        It doesn't accept 0x or 0X prefix.",
        "<volume-id>",
    );
    opts.optopt(
        "n",
        "",
        "Volume name (label), up to 15 characters. By default no label is set.",
        "<volume-name>",
    );
    opts.optopt(
        "p",
        "",
        "First sector of the partition starting from the beginning of \
        the whole disk. exFAT super block has a field for this value but in fact \
        it's optional and does not affect anything. Default is 0.",
        "<partition-first-sector>",
    );
    opts.optopt(
        "s",
        "",
        "Number of physical sectors per cluster (cluster is an allocation unit in exFAT). \
        Must be a power of 2, i.e. 1, 2, 4, 8, etc. Cluster size can not exceed 32 MB. \
        Default cluster sizes are: 4 KB if volume size is less than 256 MB, \
        32 KB if volume size is from 256 MB to 32 GB, \
        128 KB if volume size is 32 GB or larger.",
        "<sectors-per-cluster>",
    );
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
        print_version();
        std::process::exit(0);
    }
    if matches.opt_present("help") {
        usage(prog, &opts);
        std::process::exit(0);
    }

    // XXX relan/exfat allows 0x prefix
    let volume_serial = match matches.opt_str("i") {
        Some(v) => match u32::from_str_radix(&v, 16) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        },
        None => 0,
    };
    let volume_label = matches.opt_str("n").unwrap_or_default();
    let first_sector = match matches.opt_str("p") {
        Some(v) => match v.parse::<u64>() {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        },
        None => 0,
    };
    let spc_bits = match matches.opt_str("s") {
        Some(v) => match v.parse::<i32>() {
            Ok(x) => {
                let spc_bits = logarithm2(x);
                if spc_bits < 0 {
                    log::error!("invalid option value: '{v}'");
                    std::process::exit(1);
                }
                spc_bits
            }
            Err(e) => {
                log::error!("{e}");
                std::process::exit(1);
            }
        },
        None => -1,
    };
    let debug = matches.opt_present("debug");

    if let Err(e) = exfat_utils::util::init_std_logger(debug) {
        log::error!("{e}");
        std::process::exit(1);
    }

    let args = matches.free;
    if args.len() != 1 {
        usage(prog, &opts);
        std::process::exit(1);
    }

    let mut dev = match libexfat::open(&args[0], "rw") {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = setup(
        &mut dev,
        9,
        spc_bits,
        &volume_label,
        volume_serial,
        first_sector,
    ) {
        log::error!("{e}");
        std::process::exit(1);
    }
    println!("File system created successfully.");
}
