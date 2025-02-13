pub(crate) struct FsObject {
    param: crate::MkfsParam,
}

impl FsObject {
    fn init_label_entry(&self) -> exfat_utils::Result<libexfat::fs::ExfatEntryLabel> {
        let mut label = libexfat::fs::ExfatEntryLabel::new();
        label.typ = libexfat::fs::EXFAT_ENTRY_LABEL ^ libexfat::fs::EXFAT_ENTRY_VALID;
        assert!(self.param.volume_label.len() <= libexfat::fs::EXFAT_ENAME_MAX);
        if libexfat::utf::utf16_length(&self.param.volume_label) == 0 {
            return Ok(label);
        }
        label
            .name
            .copy_from_slice(&self.param.volume_label[..libexfat::fs::EXFAT_ENAME_MAX]);
        label.length = libexfat::utf::utf16_length(&label.name).try_into()?;
        label.typ |= libexfat::fs::EXFAT_ENTRY_VALID;
        Ok(label)
    }

    fn init_bitmap_entry(
        &self,
        fmap: &std::collections::HashMap<
            crate::mkexfat::FsObjectType,
            Box<dyn crate::mkexfat::FsObjectTrait>,
        >,
    ) -> exfat_utils::Result<libexfat::fs::ExfatEntryBitmap> {
        let mut bitmap = libexfat::fs::ExfatEntryBitmap::new();
        bitmap.typ = libexfat::fs::EXFAT_ENTRY_BITMAP;
        bitmap.start_cluster = libexfat::fs::EXFAT_FIRST_DATA_CLUSTER.to_le();
        bitmap.size = crate::mkexfat::get_fso!(fmap, &crate::mkexfat::FsObjectType::Cbm)
            .get_size(fmap)?
            .to_le();
        Ok(bitmap)
    }

    fn init_upcase_entry(
        &self,
        fmap: &std::collections::HashMap<
            crate::mkexfat::FsObjectType,
            Box<dyn crate::mkexfat::FsObjectTrait>,
        >,
    ) -> exfat_utils::Result<libexfat::fs::ExfatEntryUpcase> {
        let mut sum = 0u32;
        for i in 0..crate::uctc::UPCASE_TABLE.len() {
            sum = sum.rotate_right(1) + u32::from(crate::uctc::UPCASE_TABLE[i]);
        }
        let mut upcase = libexfat::fs::ExfatEntryUpcase::new();
        upcase.typ = libexfat::fs::EXFAT_ENTRY_UPCASE;
        upcase.checksum = sum.to_le();
        upcase.start_cluster = (u32::try_from(
            (crate::mkexfat::get_position(&crate::mkexfat::FsObjectType::Uct, fmap)?
                - crate::mkexfat::get_position(&crate::mkexfat::FsObjectType::Cbm, fmap)?)
                / self.param.cluster_size,
        )? + libexfat::fs::EXFAT_FIRST_DATA_CLUSTER)
            .to_le();
        upcase.size = u64::try_from(std::mem::size_of_val(&crate::uctc::UPCASE_TABLE))?.to_le();
        Ok(upcase)
    }
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
        Ok(self.param.cluster_size)
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
        let mut offset = offset;

        let label = self.init_label_entry()?;
        let buf: &[u8; libexfat::fs::EXFAT_ENTRY_SIZE] = bytemuck::cast_ref(&label);
        dev.pwrite(buf, offset)?;
        offset += u64::try_from(buf.len())?;

        let bitmap = self.init_bitmap_entry(fmap)?;
        let buf: &[u8; libexfat::fs::EXFAT_ENTRY_SIZE] = bytemuck::cast_ref(&bitmap);
        dev.pwrite(buf, offset)?;
        offset += u64::try_from(buf.len())?;

        let upcase = self.init_upcase_entry(fmap)?;
        let buf: &[u8; libexfat::fs::EXFAT_ENTRY_SIZE] = bytemuck::cast_ref(&upcase);
        Ok(dev.pwrite(buf, offset)?)
    }
}
