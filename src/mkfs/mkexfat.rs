use crate::cbm;
use crate::fat;
use crate::rootdir;
use crate::uct;
use crate::vbr;
use crate::MkfsParam;

use std::io::Write;
use std::slice::Iter;

macro_rules! get_fso {
    ($fmap:expr, $fst:expr) => {
        $fmap.get($fst).unwrap()
    };
}
pub(crate) use get_fso;

// relan/exfat uses the same address for both VBR's,
// causing get_position() to return the same position.
#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) enum FsObjectType {
    Vbr1,
    Vbr2,
    Fat,
    Cbm,
    Uct,
    Rootdir,
}

impl FsObjectType {
    fn iterator() -> Iter<'static, FsObjectType> {
        static I: [FsObjectType; 6] = [
            FsObjectType::Vbr1,
            FsObjectType::Vbr2,
            FsObjectType::Fat,
            FsObjectType::Cbm,
            FsObjectType::Uct,
            FsObjectType::Rootdir,
        ];
        I.iter()
    }
}

pub(crate) trait FsObjectTrait {
    fn new(param: MkfsParam) -> Self
    where
        Self: Sized;
    fn get_alignment(&self) -> u64;
    fn get_size(
        &self,
        fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
    ) -> u64;
    fn write(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
    ) -> std::io::Result<()>;
}

fn alloc_fsobject(
    param: &MkfsParam,
) -> std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>> {
    let mut fmap: std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>> =
        std::collections::HashMap::new();
    fmap.insert(FsObjectType::Vbr1, Box::new(vbr::FsObject::new(*param)));
    fmap.insert(FsObjectType::Vbr2, Box::new(vbr::FsObject::new(*param)));
    fmap.insert(FsObjectType::Fat, Box::new(fat::FsObject::new(*param)));
    fmap.insert(FsObjectType::Cbm, Box::new(cbm::FsObject::new(*param))); // clusters heap
    fmap.insert(FsObjectType::Uct, Box::new(uct::FsObject::new(*param))); // clusters heap
    fmap.insert(
        FsObjectType::Rootdir,
        Box::new(rootdir::FsObject::new(*param)),
    ); // clusters heap
    fmap
}

fn debug(
    param: &MkfsParam,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) {
    log::debug!("param {param:?}");
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        log::debug!(
            "{:?} alignment {:#x} size {:#x} position {:#x}",
            t,
            f.get_alignment(),
            f.get_size(fmap),
            get_position(t, fmap),
        );
    }
}

fn check_size(
    volume_size: u64,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> std::io::Result<()> {
    let mut position = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        position += f.get_size(fmap);
    }
    if position > volume_size {
        let (value, unit) = libexfat::util::humanize_bytes(volume_size);
        log::error!("too small device ({value} {unit})");
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    Ok(())
}

fn erase_object(
    dev: &mut libexfat::device::ExfatDevice,
    block: &[u8],
    block_size: u64,
    start: u64,
    size: u64,
) -> std::io::Result<()> {
    if let Err(e) = dev.seek_set(start) {
        log::error!("seek to {start:#x} failed");
        return Err(e);
    }
    let mut i = 0;
    while i < size {
        let buf = &block[..std::cmp::min(size - i, block_size).try_into().unwrap()];
        if let Err(e) = dev.write(buf) {
            log::error!(
                "failed to erase block {}/{} at {:#x}",
                i + 1,
                libexfat::div_round_up!(size, block_size),
                start
            );
            return Err(e);
        }
        i += block_size;
    }
    Ok(())
}

fn erase(
    dev: &mut libexfat::device::ExfatDevice,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> std::io::Result<()> {
    let block_size = 1024 * 1024;
    let block = vec![0; block_size];
    let mut position = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        erase_object(
            dev,
            &block,
            block_size.try_into().unwrap(),
            position,
            f.get_size(fmap),
        )?;
        position += f.get_size(fmap);
    }
    Ok(())
}

fn create(
    dev: &mut libexfat::device::ExfatDevice,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> std::io::Result<()> {
    let mut position = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        if let Err(e) = dev.seek_set(position) {
            log::error!("seek to {position:#x} failed");
            return Err(e);
        }
        f.write(dev, fmap)?;
        position += f.get_size(fmap);
    }
    Ok(())
}

pub(crate) fn mkfs(
    dev: &mut libexfat::device::ExfatDevice,
    param: &MkfsParam,
) -> std::io::Result<()> {
    let fmap = alloc_fsobject(param);
    debug(param, &fmap);
    check_size(param.volume_size, &fmap)?;

    print!("Creating... ");
    std::io::stdout().flush().unwrap();
    erase(dev, &fmap)?;
    create(dev, &fmap)?;
    println!("done.");

    print!("Flushing... ");
    std::io::stdout().flush().unwrap();
    dev.fsync()?;
    println!("done.");

    Ok(())
}

pub(crate) fn get_position(
    fst: &FsObjectType,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> u64 {
    let mut position = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        if *t == *fst {
            return position;
        }
        position += f.get_size(fmap);
    }
    panic!("unknown object");
}
