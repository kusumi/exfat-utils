pub(crate) struct FsObject {
    param: crate::MkfsParam,
}

impl crate::mkexfat::FsObjectTrait for FsObject {
    fn new(param: crate::MkfsParam) -> Self {
        Self { param }
    }

    fn get_alignment(&self) -> u64 {
        self.param.cluster_size
    }

    fn get_size(
        &self,
        _fmap: &std::collections::HashMap<
            crate::mkexfat::FsObjectType,
            Box<dyn crate::mkexfat::FsObjectTrait>,
        >,
    ) -> exfat_utils::Result<u64> {
        Ok(u64::try_from(std::mem::size_of_val(
            &crate::uctc::UPCASE_TABLE,
        ))?)
    }

    fn write(
        &self,
        dev: &mut libexfat::device::Device,
        offset: u64,
        fmap: &std::collections::HashMap<
            crate::mkexfat::FsObjectType,
            Box<dyn crate::mkexfat::FsObjectTrait>,
        >,
    ) -> exfat_utils::Result<()> {
        if let Err(e) = dev.pwrite(&crate::uctc::UPCASE_TABLE, offset) {
            log::error!(
                "failed to write upcase table of {} bytes",
                self.get_size(fmap)?
            );
            return Err(Box::new(e));
        }
        Ok(())
    }
}
