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
        fmap: &std::collections::HashMap<
            crate::mkexfat::FsObjectType,
            Box<dyn crate::mkexfat::FsObjectTrait>,
        >,
    ) -> exfat_utils::Result<u64> {
        Ok(libexfat::div_round_up!(
            (self.param.volume_size
                - crate::mkexfat::get_position(&crate::mkexfat::FsObjectType::Cbm, fmap)?)
                / self.param.cluster_size,
            u64::try_from(crate::CHAR_BIT)?
        ))
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
        let cbm = crate::mkexfat::get_fso!(fmap, &crate::mkexfat::FsObjectType::Cbm);
        let uct = crate::mkexfat::get_fso!(fmap, &crate::mkexfat::FsObjectType::Uct);
        let rootdir = crate::mkexfat::get_fso!(fmap, &crate::mkexfat::FsObjectType::Rootdir);

        let allocated_clusters = usize::try_from(
            libexfat::div_round_up!(cbm.get_size(fmap)?, self.param.cluster_size)
                + libexfat::div_round_up!(uct.get_size(fmap)?, self.param.cluster_size)
                + libexfat::div_round_up!(rootdir.get_size(fmap)?, self.param.cluster_size),
        )?;
        let count = libexfat::round_up!(allocated_clusters, crate::CHAR_BIT);
        let mut bitmap = libfs::bitmap::Bitmap::new(count)?;
        for i in 0..count {
            if i < allocated_clusters {
                bitmap.set(i)?;
            }
        }

        if let Err(e) = dev.pwrite(bitmap.as_bytes(), offset) {
            log::error!(
                "failed to write bitmap of {} bytes",
                count / crate::CHAR_BIT
            );
            return Err(Box::new(e));
        }
        Ok(())
    }
}
