pub mod core;

#[cfg(test)]
mod tests {
	use core::panic;
	use std::fs::File;
	use crate::core::FromFile;

	#[test]
	fn test_stuff() {
		let mut f = File::open("ref.ttf").unwrap();
		let mut font = crate::core::OTTF::from_file(&mut f).unwrap();
		let mut name = None;
		for r in (*font.table_directory.table_records).iter_mut(){
			if r.table_tag.data.as_str() == "name"{
				name = Some(r);
			}
		}
		let name = name.unwrap();
		panic!("{:#?}",name.get_table(&mut f));
	}
}