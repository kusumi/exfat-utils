exfat-utils ([v0.1.0](https://github.com/kusumi/exfat-utils/releases/tag/v0.1.0))
========

## About

Rust version of exfat-utils in [https://github.com/relan/exfat](https://github.com/relan/exfat)

## Supported platforms

Linux

## Requirements

Rust 1.79.0 or newer

## Build

    $ make

## Usage

dumpexfat

    $ ./target/release/dumpexfat
    Usage: ./target/release/dumpexfat [-s] [-u] [-f file] [-V] <device>
    
    Options:
        -s                  Dump only info from super block. May be useful for
                            heavily corrupted file systems.
        -u                  Dump ranges of used sectors starting from 0 and
                            separated with spaces. May be useful for backup tools.
        -f <file>           Print out a list of fragments that compose the given
                            file. Each fragment is printed on its own line, as the
                            start offset (in bytes) into the file system, and the
                            length (in bytes).
        -V, --version       Print version and copyright.
        -h, --help          Print usage.
            --debug

exfatattrib

    $ ./target/release/exfatattrib
    Usage: ./target/release/exfatattrib -d <device> <file>
           ./target/release/exfatattrib [FLAGS] -d <device> <file>
    
    Options:
        -d <device>         The path to an unmounted disk partition or disk image
                            file containing an exFAT file system. This option is
                            required.
        -r                  Set read-only flag
        -R                  Clear read-only flag
        -i                  Set hidden flag (mnemonic: invisible)
        -I                  Clear hidden flag
        -s                  Set system flag
        -S                  Clear system flag
        -a                  Set archive flag
        -A                  Clear archive flag
        -V, --version       Print version and copyright.
        -h, --help          Print usage.
            --debug

exfatfsck

    $ ./target/release/exfatfsck
    exfatfsck 1.4.0
    Usage: ./target/release/exfatfsck [-a | -n | -p | -y] [-V] <device>
    
    Options:
        -a                  Automatically repair the file system. No user
                            intervention required.
        -n                  No-operation mode: non-interactively check for errors,
                            but don't write anything to the file system.
        -p                  Same as -a for compatibility with other *fsck.
        -y                  Same as -a for compatibility with other *fsck.
        -V, --version       Print version and copyright.
        -h, --help          Print usage.
            --debug

exfatlabel

    $ ./target/release/exfatlabel
    Usage: ./target/release/exfatlabel [-V] <device> [label]
    
    Options:
        -V, --version       Print version and copyright.
        -h, --help          Print usage.
            --debug

mkexfatfs

    $ ./target/release/mkexfatfs
    mkexfatfs 1.4.0
    Usage: ./target/release/mkexfatfs [-i volume-id] [-n label] [-p partition-first-sector] [-s sectors-per-cluster] [-V] <device>
    
    Options:
        -i <volume-id>      A 32-bit hexadecimal number. By default a value based
                            on current time is set. It doesn't accept 0x or 0X
                            prefix.
        -n <volume-name>    Volume name (label), up to 15 characters. By default
                            no label is set.
        -p <partition-first-sector>
                            First sector of the partition starting from the
                            beginning of the whole disk. exFAT super block has a
                            field for this value but in fact it's optional and
                            does not affect anything. Default is 0.
        -s <sectors-per-cluster>
                            Number of physical sectors per cluster (cluster is an
                            allocation unit in exFAT). Must be a power of 2, i.e.
                            1, 2, 4, 8, etc. Cluster size can not exceed 32 MB.
                            Default cluster sizes are: 4 KB if volume size is less
                            than 256 MB, 32 KB if volume size is from 256 MB to 32
                            GB, 128 KB if volume size is 32 GB or larger.
        -V, --version       Print version and copyright.
        -h, --help          Print usage.
            --debug

modexfatfs

    $ ./target/release/modexfatfs
    modexfatfs 1.4.0
    Usage: ./target/release/modexfatfs [-c "fail"|"ignore"|"unlink"] [-V] <device> <directory> [<extra-directory>...]
    
    Options:
        -c, --conflict <"fail"|"ignore"|"unlink">
                            Action to take when a given path already exists within
                            <device>. "fail" fails with EEXIST unless both paths
                            are directory. "ignore" ignores a given path and
                            leaves the existing path as is. "unlink" unlinks the
                            existing path first and then create. Unlink of
                            directory (and its child entries) is unsupported.
                            Defaults to "fail".
        -V, --version       Print version and copyright.
        -h, --help          Print usage.
            --debug

## License

[GPLv2](COPYING)

Copyright (C) 2010-  Andrew Nayenko

Copyright (C) 2024-  Tomohiro Kusumi
