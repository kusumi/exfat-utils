use crate::mkexfat;
use crate::uctc;
use crate::MkfsParam;

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
        _fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> u64 {
        u64::try_from(std::mem::size_of_val(&uctc::UPCASE_TABLE)).unwrap()
    }

    fn write(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        offset: u64,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> std::io::Result<()> {
        if let Err(e) = dev.pwrite(&uctc::UPCASE_TABLE, offset) {
            log::error!(
                "failed to write upcase table of {} bytes",
                self.get_size(fmap)
            );
            return Err(e);
        }
        Ok(())
    }
}
