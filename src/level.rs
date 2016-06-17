use byteorder::{LittleEndian as E, ReadBytesExt};
use splay::Splay;

pub const NUM_TERRAINS: usize = 8;

pub struct Power(pub i32);
impl Power {
    fn as_value(&self) -> i32 {
        1 << self.0
    }
    fn as_power(&self) -> i32 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct TerrainConfig {
    pub shadow_offset: u8,
    pub height_shift: u8,
    pub color_range: (u8, u8),
}

pub struct LevelConfig {
    pub name: String,
    pub path_palette: String,
    pub path_vpr: String,
    pub path_vmc: String,
    pub is_compressed: bool,
    pub size: (Power, Power),
    pub geo: Power,
    pub section: Power,
    pub min_square: Power,
    pub terrains: [TerrainConfig; NUM_TERRAINS],
}

pub struct Level {
    pub size: (i32, i32),
    pub flood_map: Vec<u32>,
    pub height: Vec<u8>,
    pub meta: Vec<u8>,
    pub palette: [[u8; 4]; 0x100],
    pub terrains: [TerrainConfig; NUM_TERRAINS],
}

pub fn load(config: &LevelConfig) -> Level {
    use std::fs::File;
    use std::io::{BufReader, Read, Seek, SeekFrom};

    assert!(config.is_compressed);
    let size = (config.size.0.as_value(), config.size.1.as_value());

    info!("Loading vpr...");
    let flood = {
        let vpr_file = File::open(&config.path_vpr).unwrap();
        let flood_size = size.1 >> config.section.as_power();
        let geo_pow = config.geo.as_power();
        let net_size = size.0 * size.1 >> (2 * geo_pow);
        let flood_offset = (2*4 + (1 + 4 + 4)*4 + 2*net_size + 2*geo_pow*4 + 2*flood_size*geo_pow*4) as u64;
        let expected_file_size = flood_offset + (flood_size*4) as u64;
        assert_eq!(vpr_file.metadata().unwrap().len(), expected_file_size as u64);
        let mut vpr = BufReader::new(vpr_file);
        vpr.seek(SeekFrom::Start(flood_offset)).unwrap();
        (0..flood_size).map(|_|
            vpr.read_u32::<E>().unwrap()
        ).collect()
    };
    
    info!("Loading vmc...");
    let (height, meta) = {
        use progressive::progress;

        let mut vpc = BufReader::new(File::open(&config.path_vmc).unwrap());
        info!("\tLoading compression tables...");
        let mut st_table = Vec::<i32>::with_capacity(size.1 as usize);
        let mut sz_table = Vec::<i16>::with_capacity(size.1 as usize);
        for _ in 0 .. size.1 {
            st_table.push(vpc.read_i32::<E>().unwrap());
            sz_table.push(vpc.read_i16::<E>().unwrap());
        }
        info!("\tDecompressing level data...");
        let splay = Splay::new(&mut vpc);
        let total = (size.0 * size.1) as usize;
        let mut height = Vec::with_capacity(total);
        let mut meta = Vec::with_capacity(total);
        for y in progress(0 .. size.1) {
            vpc.seek(SeekFrom::Start(st_table[y as usize] as u64)).unwrap();
            let target_size = ((y+1) * size.0) as usize;
            while height.len() < target_size && meta.len() < target_size {
                splay.expand1(&mut vpc, &mut height);
                splay.expand2(&mut vpc, &mut meta);
            }
            assert_eq!(height.len(), target_size);
            assert_eq!(meta.len(), target_size);
        }
        (height, meta)
    };

    info!("Loading palette...");
    let pal = {
        let mut pal_file = BufReader::new(File::open(&config.path_palette).unwrap());
        let mut pal = [[0; 4]; 0x100];
        for p in pal.iter_mut() {
            pal_file.read(&mut p[..3]).unwrap();
        }
        pal
    };

    info!("Done.");
    Level {
        size: size,
        flood_map: flood,
        height: height,
        meta: meta,
        palette: pal,
        terrains: config.terrains,
    }
}
