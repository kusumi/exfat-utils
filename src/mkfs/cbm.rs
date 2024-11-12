use crate::mkexfat;
use crate::MkfsParam;
use crate::CHAR_BIT;

pub(crate) struct FsObject {
    param: MkfsParam,
}

impl mkexfat::FsObjectTrait for FsObject {
    fn new(param: MkfsParam) -> Self {
        Self { param }
    }

    fn get_alignment(&self) -> u64 {
        self.param.cluster_size
    }

    fn get_size(
        &self,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> u64 {
        libexfat::div_round_up!(
            (self.param.volume_size - mkexfat::get_position(&mkexfat::FsObjectType::Cbm, fmap))
                / self.param.cluster_size,
            u64::try_from(CHAR_BIT).unwrap()
        )
    }

    fn write(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        offset: u64,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> std::io::Result<()> {
        let cbm = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Cbm);
        let uct = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Uct);
        let rootdir = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Rootdir);

        let allocated_clusters = usize::try_from(
            libexfat::div_round_up!(cbm.get_size(fmap), self.param.cluster_size)
                + libexfat::div_round_up!(uct.get_size(fmap), self.param.cluster_size)
                + libexfat::div_round_up!(rootdir.get_size(fmap), self.param.cluster_size),
        )
        .unwrap();
        let count = libexfat::round_up!(allocated_clusters, CHAR_BIT);
        let mut bitmap = libexfat::bitmap::alloc(count);
        for i in 0..count {
            if i < allocated_clusters {
                libexfat::bitmap::set(&mut bitmap, i);
            }
        }

        if let Err(e) = dev.pwrite(&bitmap, offset) {
            log::error!("failed to write bitmap of {} bytes", count / CHAR_BIT);
            return Err(e);
        }
        Ok(())
    }
}
