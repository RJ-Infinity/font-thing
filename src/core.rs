use std::io::{Read, Seek, SeekFrom};

#[derive(Debug)]
pub enum FromFileErr<InvalidData,OtherType>{
	EOF,
	InvalidData(InvalidData),
	Other(OtherType)
}
macro_rules! from_file_def {($f: ident, $body: block) => {
	fn from_file<F>($f: &mut F)->Result<Self, FromFileErr<InvalidData,OtherType>> where
		Self: Sized,
		F: Read,
		F: Seek
	$body
};}
pub trait FromFile<InvalidData,OtherType>{
	fn from_file<F>(f: &mut F)->Result<Self, FromFileErr<InvalidData,OtherType>> where
		Self: Sized,
		F: Read,
		F: Seek
	;
}

macro_rules! unwrap_or_ret {($val: expr) => {match $val{
	Ok(v)=>v,
	Err(e)=>return Err(e),
}};}

macro_rules! val_or_ret {($val: expr, $validator: expr, $on_err: expr) => {{
	let v = $val;
	if $validator(v){v}else{return Err(FromFileErr::InvalidData($on_err))}
}};}

impl FromFile<(),()> for u32{from_file_def!(f, {
	Ok((unwrap_or_ret!(u16::from_file(f)) as u32) << 16 | unwrap_or_ret!(u16::from_file(f)) as u32)
});}
impl FromFile<(),()> for u16{from_file_def!(f, {
	Ok((unwrap_or_ret!(u8::from_file(f)) as u16) << 8 | unwrap_or_ret!(u8::from_file(f)) as u16)
});}
impl FromFile<(),()> for u8{from_file_def!(f, {
	let mut buf = [0u8];
	match f.read(buf.as_mut_slice()){
		Ok(1) => Ok(buf[0]),
		Ok(0) => Err(FromFileErr::EOF),
		Ok(_) => unreachable!(),
		Err(e) => {
			panic!("{}",e);
			Err(FromFileErr::Other(()))
		}
	}
});}

#[derive(Debug)]
pub struct OTTF{
	pub table_directory: TableDirectory,
}
impl FromFile<(),()> for OTTF{from_file_def!(f, {Ok(Self{
	table_directory: unwrap_or_ret!(TableDirectory::from_file(f)),
})});}

#[derive(Debug)]
pub enum SFNTVer{
	TrueType,
	CFF,
	Unknown(u32),
}
impl SFNTVer{fn from_u32(v: u32)->Self{match v{
	0x00010000 => Self::TrueType,
	0x4F54544F => Self::CFF,
	v => Self::Unknown(v),
}}}

#[derive(Debug)]
pub struct TableDirectory{
	/// 0x00010000 or 0x4F54544F ('OTTO')
	pub sfnt_version: SFNTVer,
	/// Number of tables.
	pub num_tables: u16,
	/// Maximum power of 2 less than or equal to numTables, times 16 ((2**floor(log2(numTables))) * 16).
	pub search_range: u16,
	/// Log2 of the maximum power of 2 less than or equal to numTables (log2(searchRange/16), which is equal to floor(log2(numTables))).
	pub entry_selector: u16,
	/// numTables times 16, minus searchRange ((numTables * 16) - searchRange).
	pub range_shift: u16,
	/// this should have the count of num_tables
	pub table_records: Box<[TableRecord]>,
}
impl FromFile<(),()> for TableDirectory{from_file_def!(f, {
	let sfnt_version = SFNTVer::from_u32(unwrap_or_ret!(u32::from_file(f)));
	let num_tables = unwrap_or_ret!(u16::from_file(f));
	return Ok(Self{
		sfnt_version,
		num_tables,
		search_range: unwrap_or_ret!(u16::from_file(f)),
		entry_selector: unwrap_or_ret!(u16::from_file(f)),
		range_shift: unwrap_or_ret!(u16::from_file(f)),
		table_records: {
			let mut rec = vec![];
			for _ in 0..num_tables{rec.push(unwrap_or_ret!(TableRecord::from_file(f)))}
			rec.into()
		},
	});
});}

#[derive(Debug)]
pub struct TableRecord{
	pub table_tag: Tag,
	pub checksum: u32,
	pub offset: Offset32,
	pub length: u32,
	cached_table: Option<Result<Box<Table>, FromFileErr<(),Box<[u8]>>>>,
}
impl FromFile<(),()> for TableRecord{from_file_def!(f, {Ok(Self{
	table_tag: unwrap_or_ret!(Tag::from_file(f)),
	checksum: unwrap_or_ret!(u32::from_file(f)),
	offset: unwrap_or_ret!(Offset32::from_file(f)),
	length: unwrap_or_ret!(u32::from_file(f)),
	cached_table: None,
})});}
impl TableRecord{
	pub fn get_table<T>(&mut self, f: &mut T)->&Result<Box<Table>, FromFileErr<(),Box<[u8]>>> where T:Read, T:Seek{
		if self.cached_table.is_none(){
			let cur = f.seek(SeekFrom::Current(0));
			if f.seek(SeekFrom::Start(self.offset as u64)).is_err()
			{return &Err(FromFileErr::EOF);}

			self.cached_table = Some(match self.table_tag.data.as_str(){
				"name" => Ok(Table::Name(match NameTable::from_file(f){
					Ok(v) => v,
					Err(e) => match e{
						FromFileErr::EOF => return &Err(FromFileErr::EOF),
						FromFileErr::InvalidData(_) => return &Err(FromFileErr::InvalidData(())),
						FromFileErr::Other(_) => unreachable!(),
					},
				}).into()),
				_ => Err(FromFileErr::Other([].into())),
			});
			f.seek(SeekFrom::Start(cur.unwrap())).unwrap();
		}
		self.cached_table.as_ref().unwrap()
	}
}

#[derive(Debug)]
pub struct Tag{
	pub data: String,
}
impl FromFile<(),()> for Tag{from_file_def!(f, {
	let in_range = |chr: u8|{chr>=0x20&&chr<=0x7E};
	Ok(Self{data: String::from_iter([
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
	])})
});}

type Offset32 = u32;
type Offset16 = u16;

#[derive(Debug)]
pub enum Table{
	Name(NameTable)
}

#[derive(Debug)]
pub struct NameTable{
	pub version: u16,
	pub count: u16,
	pub storage_offset: Offset16,
	pub name_records: Box<[NameRecord]>,
	pub lang_tag_count: Option<u16>,
	pub lang_tag_record: Option<Box<[LangTagRecord]>>,
}
impl FromFile<(),()> for NameTable{from_file_def!(f, {
	let version = unwrap_or_ret!(u16::from_file(f));
	let count = unwrap_or_ret!(u16::from_file(f));
	let storage_offset = unwrap_or_ret!(Offset16::from_file(f));
	let name_records = {
		let mut rec = vec![];
		for _ in 0..count{rec.push(unwrap_or_ret!(NameRecord::from_file(f)))}
		rec.into()
	};
	let lang_tag_count = if version == 0{None}else{Some(unwrap_or_ret!(u16::from_file(f)))};
	return Ok(Self{
		version,
		count,
		storage_offset,
		name_records,
		lang_tag_count,
		lang_tag_record: match lang_tag_count{Some(lang_tag_count)=>{
			let mut rec = vec![];
			for _ in 0..lang_tag_count{rec.push(unwrap_or_ret!(LangTagRecord::from_file(f)))}
			Some(rec.into())
		},None=>None},
	});
});}
#[derive(Debug)]
pub struct LangTagRecord{
	///Language-tag string length (in bytes)
	pub length: u16,
	///Language-tag string offset from start of storage area (in bytes).
	pub lang_tag_offset: Offset16,
}
impl FromFile<(),()> for LangTagRecord{from_file_def!(f, {Ok(Self{
	length: unwrap_or_ret!(u16::from_file(f)),
	lang_tag_offset: unwrap_or_ret!(Offset16::from_file(f)),
})});}
#[derive(Debug)]
pub struct NameRecord{
	///Platform ID.
	pub platform_id: u16,
	///Platform-specific encoding ID.
	pub encoding_id: u16,
	///Language ID.
	pub language_id: u16,
	///Name ID.
	pub name_id: u16,
	///String length (in bytes).
	pub length: u16,
	///String offset from start of storage area (in bytes).
	pub string_offset: Offset16,
}
impl FromFile<(),()> for NameRecord{from_file_def!(f, {Ok(Self{
	platform_id: unwrap_or_ret!(u16::from_file(f)),
	encoding_id: unwrap_or_ret!(u16::from_file(f)),
	language_id: unwrap_or_ret!(u16::from_file(f)),
	name_id: unwrap_or_ret!(u16::from_file(f)),
	length: unwrap_or_ret!(u16::from_file(f)),
	string_offset: unwrap_or_ret!(Offset16::from_file(f)),
})});}

pub fn calc_table_checksum<T>(table: T, length: u32) -> u32 where T: Fn(usize) -> u32{
	let mut sum = 0u32;
	let endptr = ((length+3) & !3) / 4;
	let mut i = 0;
	while i < endptr as usize {
		sum += table(i);
		i+=1;
	}
	return sum;
}