use byteorder::ByteOrder;

pub(crate) struct FsObject {
    param: crate::MkfsParam,
}

impl FsObject {
    fn fat_write_entry(
        dev: &mut libexfat::device::Device,
        offset: u64,
        cluster: u32,
        value: u32,
    ) -> exfat_utils::Result<(u64, u32)> {
        let fat_entry = value.to_le();
        let mut buf = vec![0; 4];
        byteorder::LittleEndian::write_u32_into(&[fat_entry], &mut buf);
        dev.pwrite(&buf, offset)?;
        Ok((offset + u64::try_from(buf.len())?, cluster + 1))
    }

    fn fat_write_entries(
        &self,
        dev: &mut libexfat::device::Device,
        offset: u64,
        cluster: u32,
        length: u64,
    ) -> exfat_utils::Result<(u64, u32)> {
        let end =
            cluster + u32::try_from(libexfat::div_round_up!(length, self.param.cluster_size))?;
        let mut offset = offset;
        let mut cluster = cluster;
        while cluster < end - 1 {
            let t = Self::fat_write_entry(dev, offset, cluster, cluster + 1)?;
            offset = t.0;
            cluster = t.1;
            if cluster == 0 {
                return Err(Box::new(nix::errno::Errno::EINVAL));
            }
        }
        Self::fat_write_entry(dev, offset, cluster, libexfat::fs::EXFAT_CLUSTER_END)
    }
}

impl crate::mkexfat::FsObjectTrait for FsObject {
    fn new(param: crate::MkfsParam) -> Self {
        Self { param }
    }

    fn get_alignment(&self) -> u64 {
        128 * self.param.sector_size
    }

    fn get_size(
        &self,
        _fmap: &std::collections::HashMap<
            crate::mkexfat::FsObjectType,
            Box<dyn crate::mkexfat::FsObjectTrait>,
        >,
    ) -> exfat_utils::Result<u64> {
        Ok(self.param.volume_size / self.param.cluster_size
            * u64::try_from(std::mem::size_of::<u32>())?)
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

        let (o, c) = Self::fat_write_entry(dev, offset, 0, 0xffff_fff8)?; // media type
        let (o, c) = Self::fat_write_entry(dev, o, c, 0xffff_ffff)?; // some weird constant
        let (o, c) = self.fat_write_entries(dev, o, c, cbm.get_size(fmap)?)?;
        let (o, c) = self.fat_write_entries(dev, o, c, uct.get_size(fmap)?)?;
        self.fat_write_entries(dev, o, c, rootdir.get_size(fmap)?)?;
        Ok(())
    }
}
