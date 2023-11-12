pub mod core;
pub mod char_sets;

#[cfg(test)]
mod tests {
	use core::panic;
	use std::fs::File;
	use crate::{core::FromFile, char_sets::{CharSetStr, CodePage437, MacOsRoman, Utf16, Utf8}};

	#[test]
	fn test_stuff() {
		let mut f = File::open("consola.ttf").unwrap();
		let mut font = crate::core::OTTF::from_file(&mut f).unwrap();
		let mut name = None;
		for r in (*font.table_directory.table_records).iter_mut(){
			if r.table_tag.data.as_str() == "name"{
				name = Some(r);
			}
		}
		let name = name.unwrap();
		match name.get_table(&mut f).unwrap(){crate::core::Table::Name(name_t) => {
			for record in name_t.name_records.iter(){println!(
				"{}@{}: {}",
				record.length,
				record.string_offset,
				record.translate_string(
					record.get_string(&mut f, &name_t).unwrap()
				).unwrap()
			);}
		},}
	}
	#[test]
	fn test_char_sets() {
		println!("{:?}",CharSetStr::<CodePage437>::from_bytes(&[1u8,2u8,3u8]));
		println!("{}",CharSetStr::<CodePage437>::from_bytes(&[1u8,2u8,3u8]).unwrap());
	}
	#[test]
	fn test_utf16(){
		println!("{:?}",CharSetStr::<Utf16>::from_bytes(&[0x00, 0x4E, 0x00, 0x6F, 0x00, 0x72, 0x00, 0x6D, 0x00, 0x61, 0x00, 0x6C,]));
	}
}