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
    fn new(param: crate::MkfsParam) -> Self
    where
        Self: Sized;
    fn get_alignment(&self) -> u64;
    fn get_size(
        &self,
        fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
    ) -> exfat_utils::Result<u64>;
    fn write(
        &self,
        dev: &mut libexfat::device::Device,
        offset: u64,
        fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
    ) -> exfat_utils::Result<()>;
}

fn alloc_fsobject(
    param: &crate::MkfsParam,
) -> std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>> {
    let mut fmap: std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>> =
        std::collections::HashMap::new();
    fmap.insert(
        FsObjectType::Vbr1,
        Box::new(crate::vbr::FsObject::new(*param)),
    );
    fmap.insert(
        FsObjectType::Vbr2,
        Box::new(crate::vbr::FsObject::new(*param)),
    );
    fmap.insert(
        FsObjectType::Fat,
        Box::new(crate::fat::FsObject::new(*param)),
    );
    fmap.insert(
        FsObjectType::Cbm,
        Box::new(crate::cbm::FsObject::new(*param)),
    ); // clusters heap
    fmap.insert(
        FsObjectType::Uct,
        Box::new(crate::uct::FsObject::new(*param)),
    ); // clusters heap
    fmap.insert(
        FsObjectType::Rootdir,
        Box::new(crate::rootdir::FsObject::new(*param)),
    ); // clusters heap
    fmap
}

fn debug(
    param: &crate::MkfsParam,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> exfat_utils::Result<()> {
    log::debug!("param {param:?}");
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        log::debug!(
            "{:?} alignment {:#x} size {:#x} position {:#x}",
            t,
            f.get_alignment(),
            f.get_size(fmap)?,
            get_position(t, fmap)?,
        );
    }
    Ok(())
}

fn check_size(
    volume_size: u64,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> exfat_utils::Result<()> {
    let mut position: u64 = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        position += f.get_size(fmap)?;
    }
    if position > volume_size {
        let (value, unit) = libexfat::util::humanize_bytes(volume_size);
        log::error!("too small device ({value} {unit})");
        return Err(Box::new(nix::errno::Errno::EINVAL));
    }
    Ok(())
}

fn erase_object(
    dev: &mut libexfat::device::Device,
    block: &[u8],
    block_size: u64,
    start: u64,
    size: u64,
) -> exfat_utils::Result<()> {
    let mut offset = start;
    let mut i = 0;
    while i < size {
        let buf = &block[..std::cmp::min(size - i, block_size).try_into()?];
        if let Err(e) = dev.pwrite(buf, offset) {
            log::error!(
                "failed to erase block {}/{} at {:#x}",
                i + 1,
                libexfat::div_round_up!(size, block_size),
                start
            );
            return Err(Box::new(e));
        }
        offset += block_size;
        i += block_size;
    }
    Ok(())
}

fn erase(
    dev: &mut libexfat::device::Device,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> exfat_utils::Result<()> {
    let block_size = 1024 * 1024;
    let block = vec![0; block_size];
    let mut position: u64 = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        erase_object(
            dev,
            &block,
            block_size.try_into()?,
            position,
            f.get_size(fmap)?,
        )?;
        position += f.get_size(fmap)?;
    }
    Ok(())
}

fn create(
    dev: &mut libexfat::device::Device,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> exfat_utils::Result<()> {
    let mut position: u64 = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        f.write(dev, position, fmap)?;
        position += f.get_size(fmap)?;
    }
    Ok(())
}

pub(crate) fn mkfs(
    dev: &mut libexfat::device::Device,
    param: &crate::MkfsParam,
) -> exfat_utils::Result<()> {
    let fmap = alloc_fsobject(param);
    debug(param, &fmap)?;
    check_size(param.volume_size, &fmap)?;

    print!("Creating... ");
    std::io::stdout().flush()?;
    erase(dev, &fmap)?;
    create(dev, &fmap)?;
    println!("done.");

    print!("Flushing... ");
    std::io::stdout().flush()?;
    dev.fsync()?;
    println!("done.");

    Ok(())
}

pub(crate) fn get_position(
    fst: &FsObjectType,
    fmap: &std::collections::HashMap<FsObjectType, Box<dyn FsObjectTrait>>,
) -> exfat_utils::Result<u64> {
    let mut position: u64 = 0;
    for t in FsObjectType::iterator() {
        let f = get_fso!(fmap, t);
        position = libexfat::round_up!(position, f.get_alignment());
        if *t == *fst {
            return Ok(position);
        }
        position += f.get_size(fmap)?;
    }
    panic!("unknown object");
}
