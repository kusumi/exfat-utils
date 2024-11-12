use crate::mkexfat;
use crate::uctc;
use crate::MkfsParam;

pub(crate) struct FsObject {
    param: MkfsParam,
}

impl FsObject {
    fn init_label_entry(&self) -> libexfat::fs::ExfatEntryLabel {
        let mut label = libexfat::fs::ExfatEntryLabel::new();
        label.typ = libexfat::fs::EXFAT_ENTRY_LABEL ^ libexfat::fs::EXFAT_ENTRY_VALID;
        assert!(self.param.volume_label.len() <= libexfat::fs::EXFAT_ENAME_MAX);
        if libexfat::utf::utf16_length(&self.param.volume_label) == 0 {
            return label;
        }
        label
            .name
            .copy_from_slice(&self.param.volume_label[..libexfat::fs::EXFAT_ENAME_MAX]);
        label.length = libexfat::utf::utf16_length(&label.name).try_into().unwrap();
        label.typ |= libexfat::fs::EXFAT_ENTRY_VALID;
        label
    }

    fn init_bitmap_entry(
        &self,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> libexfat::fs::ExfatEntryBitmap {
        let mut bitmap = libexfat::fs::ExfatEntryBitmap::new();
        bitmap.typ = libexfat::fs::EXFAT_ENTRY_BITMAP;
        bitmap.start_cluster = libexfat::fs::EXFAT_FIRST_DATA_CLUSTER.to_le();
        bitmap.size = mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Cbm)
            .get_size(fmap)
            .to_le();
        bitmap
    }

    fn init_upcase_entry(
        &self,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> libexfat::fs::ExfatEntryUpcase {
        let mut sum = 0u32;
        for i in 0..uctc::UPCASE_TABLE.len() {
            sum = sum.rotate_right(1) + u32::from(uctc::UPCASE_TABLE[i]);
        }
        let mut upcase = libexfat::fs::ExfatEntryUpcase::new();
        upcase.typ = libexfat::fs::EXFAT_ENTRY_UPCASE;
        upcase.checksum = sum.to_le();
        upcase.start_cluster = (u32::try_from(
            (mkexfat::get_position(&mkexfat::FsObjectType::Uct, fmap)
                - mkexfat::get_position(&mkexfat::FsObjectType::Cbm, fmap))
                / self.param.cluster_size,
        )
        .unwrap()
            + libexfat::fs::EXFAT_FIRST_DATA_CLUSTER)
            .to_le();
        upcase.size = u64::try_from(std::mem::size_of_val(&uctc::UPCASE_TABLE))
            .unwrap()
            .to_le();
        upcase
    }
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
        self.param.cluster_size
    }

    fn write(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        offset: u64,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> std::io::Result<()> {
        let mut offset = offset;

        let label = self.init_label_entry();
        let buf: &[u8; libexfat::fs::EXFAT_ENTRY_SIZE] = bytemuck::cast_ref(&label);
        dev.pwrite(buf, offset)?;
        offset += u64::try_from(buf.len()).unwrap();

        let bitmap = self.init_bitmap_entry(fmap);
        let buf: &[u8; libexfat::fs::EXFAT_ENTRY_SIZE] = bytemuck::cast_ref(&bitmap);
        dev.pwrite(buf, offset)?;
        offset += u64::try_from(buf.len()).unwrap();

        let upcase = self.init_upcase_entry(fmap);
        let buf: &[u8; libexfat::fs::EXFAT_ENTRY_SIZE] = bytemuck::cast_ref(&upcase);
        dev.pwrite(buf, offset)
    }
}
