use byteorder::{LittleEndian as E, ReadBytesExt};
use splay::Splay;

type Altitude = u8;

pub struct Power(pub i32);
impl Power {
	fn as_value(&self) -> i32 {
		1 << self.0
	}
	fn as_power(&self) -> i32 {
		self.0
	}
}

pub struct Config {
	pub name: String,
	pub path_vpr: String,
	pub path_vmc: String,
	pub size: (Power, Power),
	pub geo: Power,
	pub section: Power,
}

pub struct Level {
	size: (i32, i32),
	flood_map: Vec<u32>,
	data: Vec<Altitude>,
}

pub fn load(config: &Config) -> Level {
	use std::fs::File;
	use std::io::{Seek, SeekFrom};

	let size = (config.size.0.as_value(), config.size.1.as_value());

	info!("Loading vpr...");
	let flood = {
		let mut vpr = File::open(&config.path_vpr).unwrap();
		let flood_size = size.1 >> config.section.as_power();
		let geo_pow = config.geo.as_power();
		let net_size = size.0 * size.1 >> (2 * geo_pow);
		let flood_offset = (2*4 + (1 + 4 + 4)*4 + 2*net_size + 2*geo_pow*4 + 2*flood_size*geo_pow*4) as u64;
		let expected_file_size = flood_offset + (flood_size*4) as u64;
		assert_eq!(vpr.metadata().unwrap().len(), expected_file_size as u64);
		vpr.seek(SeekFrom::Start(flood_offset)).unwrap();
		(0..flood_size).map(|_|
			vpr.read_u32::<E>().unwrap()
		).collect()
	};
	
	info!("Loading vmc...");
	let data = {
		use std::io::BufReader;
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
		let mut data = Vec::with_capacity((size.0 * size.1 * 2) as usize);
		for y in progress(0 .. size.1) {
			vpc.seek(SeekFrom::Start(st_table[y as usize] as u64)).unwrap();
			let target_size = ((y+1) * 2 * size.0) as usize;
			while data.len() < target_size {
				splay.expand(&mut vpc, &mut data);
			}
			assert_eq!(data.len(), target_size);
		}
		data
	};

	info!("Done.");
	Level {
		size: size,
		flood_map: flood,
		data: data,
	}
}