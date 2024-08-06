use crate::mkexfat;
use crate::MkfsParam;

use byteorder::ByteOrder;

pub(crate) struct FsObject {
    param: MkfsParam,
}

impl FsObject {
    fn fat_write_entry(
        dev: &mut libexfat::device::ExfatDevice,
        cluster: u32,
        value: u32,
    ) -> std::io::Result<u32> {
        let fat_entry = value.to_le();
        let mut buf = vec![0; 4];
        byteorder::LittleEndian::write_u32_into(&[fat_entry], &mut buf);
        dev.write(&buf)?;
        Ok(cluster + 1)
    }

    fn fat_write_entries(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        cluster: u32,
        length: u64,
    ) -> std::io::Result<u32> {
        let end = cluster
            + u32::try_from(libexfat::div_round_up!(length, self.param.cluster_size)).unwrap();
        let mut cluster = cluster;
        while cluster < end - 1 {
            cluster = Self::fat_write_entry(dev, cluster, cluster + 1)?;
            if cluster == 0 {
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
            }
        }
        Self::fat_write_entry(dev, cluster, libexfat::exfatfs::EXFAT_CLUSTER_END)
    }
}

impl mkexfat::FsObjectTrait for FsObject {
    fn new(param: MkfsParam) -> Self {
        Self { param }
    }

    fn get_alignment(&self) -> u64 {
        128 * self.param.sector_size
    }

    fn get_size(
        &self,
        _fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> u64 {
        self.param.volume_size / self.param.cluster_size
            * u64::try_from(std::mem::size_of::<u32>()).unwrap()
    }

    fn write(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> std::io::Result<()> {
        let cbm = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Cbm);
        let uct = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Uct);
        let rootdir = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Rootdir);

        let c = Self::fat_write_entry(dev, 0, 0xffff_fff8)?; // media type
        let c = Self::fat_write_entry(dev, c, 0xffff_ffff)?; // some weird constant
        let c = self.fat_write_entries(dev, c, cbm.get_size(fmap))?;
        let c = self.fat_write_entries(dev, c, uct.get_size(fmap))?;
        self.fat_write_entries(dev, c, rootdir.get_size(fmap))?;
        Ok(())
    }
}
