use crate::mkexfat;
use crate::MkfsParam;

use byteorder::ByteOrder;

pub(crate) struct FsObject {
    param: MkfsParam,
}

impl FsObject {
    fn init_sb(
        &self,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> libexfat::fs::ExfatSuperBlock {
        let clusters_max = u32::try_from(self.param.volume_size / self.param.cluster_size).unwrap();
        let fat_sectors = u32::try_from(libexfat::div_round_up!(
            u64::from(clusters_max) * u64::try_from(std::mem::size_of::<u32>()).unwrap(),
            self.param.sector_size
        ))
        .unwrap();

        let mut sb = libexfat::fs::ExfatSuperBlock::new();
        sb.jump[0] = 0xeb;
        sb.jump[1] = 0x76;
        sb.jump[2] = 0x90;
        sb.oem_name.copy_from_slice("EXFAT   ".as_bytes());
        sb.sector_start = self.param.first_sector.to_le();
        sb.sector_count = (self.param.volume_size / self.param.sector_size).to_le();
        sb.fat_sector_start = u32::try_from(
            mkexfat::get_fso!(fmap, &mkexfat::FsObjectType::Fat).get_alignment()
                / self.param.sector_size,
        )
        .unwrap()
        .to_le();
        sb.fat_sector_count = (libexfat::round_up!(
            u32::from_le(sb.fat_sector_start) + fat_sectors,
            1 << self.param.spc_bits
        ) - u32::from_le(sb.fat_sector_start))
        .to_le();
        sb.cluster_sector_start = u32::try_from(
            mkexfat::get_position(&mkexfat::FsObjectType::Cbm, fmap) / self.param.sector_size,
        )
        .unwrap()
        .to_le();
        sb.cluster_count = (clusters_max
            - ((u32::from_le(sb.fat_sector_start) + u32::from_le(sb.fat_sector_count))
                >> self.param.spc_bits))
            .to_le();
        sb.rootdir_cluster = (u32::try_from(
            (mkexfat::get_position(&mkexfat::FsObjectType::Rootdir, fmap)
                - mkexfat::get_position(&mkexfat::FsObjectType::Cbm, fmap))
                / self.param.cluster_size,
        )
        .unwrap()
            + libexfat::fs::EXFAT_FIRST_DATA_CLUSTER)
            .to_le();
        sb.volume_serial = self.param.volume_serial.to_le();
        sb.version_major = 1;
        sb.version_minor = 0;
        sb.volume_state = 0;
        sb.sector_bits = self.param.sector_bits.try_into().unwrap();
        sb.spc_bits = self.param.spc_bits.try_into().unwrap();
        sb.fat_count = 1;
        sb.drive_no = 0x80;
        sb.allocated_percent = 0;
        sb.boot_signature = 0xaa55_u16.to_le();
        sb
    }
}

impl mkexfat::FsObjectTrait for FsObject {
    fn new(param: MkfsParam) -> Self {
        Self { param }
    }

    fn get_alignment(&self) -> u64 {
        self.param.sector_size
    }

    fn get_size(
        &self,
        _fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> u64 {
        12 * self.param.sector_size
    }

    fn write(
        &self,
        dev: &mut libexfat::device::ExfatDevice,
        offset: u64,
        fmap: &std::collections::HashMap<mkexfat::FsObjectType, Box<dyn mkexfat::FsObjectTrait>>,
    ) -> std::io::Result<()> {
        let mut offset = offset;

        let sb = self.init_sb(fmap);
        let buf = libexfat::util::any_as_u8_slice(&sb);
        if let Err(e) = dev.pwrite(buf, offset) {
            log::error!("failed to write super block sector");
            return Err(e);
        }
        offset += u64::try_from(buf.len()).unwrap();

        let mut checksum =
            libexfat::util::vbr_start_checksum(buf, libexfat::fs::EXFAT_SUPER_BLOCK_SIZE_U64);
        let mut sector = vec![0; self.param.sector_size.try_into().unwrap()];
        let n = sector.len();
        sector[n - 4] = 0;
        sector[n - 3] = 0;
        sector[n - 2] = 0x55;
        sector[n - 1] = 0xaa;

        for _ in 0..8 {
            if let Err(e) = dev.pwrite(&sector, offset) {
                log::error!("failed to write a sector with boot signature");
                return Err(e);
            }
            checksum = libexfat::util::vbr_add_checksum(&sector, self.param.sector_size, checksum);
            offset += u64::try_from(sector.len()).unwrap();
        }

        let sector = vec![0; self.param.sector_size.try_into().unwrap()];
        for _ in 0..2 {
            if let Err(e) = dev.pwrite(&sector, offset) {
                log::error!("failed to write an empty sector");
                return Err(e);
            }
            checksum = libexfat::util::vbr_add_checksum(&sector, self.param.sector_size, checksum);
            offset += u64::try_from(sector.len()).unwrap();
        }

        let mut buf = vec![0; 4];
        byteorder::LittleEndian::write_u32_into(&[checksum.to_le()], &mut buf);
        let mut sector = vec![0; self.param.sector_size.try_into().unwrap()];
        let mut i = 0;
        while i < usize::try_from(self.param.sector_size).unwrap() {
            sector[i..(i + 4)].copy_from_slice(&buf[..4]);
            i += 4;
        }

        if let Err(e) = dev.pwrite(&sector, offset) {
            log::error!("failed to write checksum sector");
            return Err(e);
        }
        Ok(())
    }
}
