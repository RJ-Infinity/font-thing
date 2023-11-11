use std::io::{Read, Seek, SeekFrom};

#[derive(Debug)]
pub enum FromFileErr<InvalidData,OtherType>{
	EOF,
	InvalidData(InvalidData),
	Other(OtherType)
}
macro_rules! impl_from_file {($type: ty, $invalid_data: ty, $other_type: ty, $f: ident, $body: block) => {
	impl FromFile<$invalid_data,$other_type> for $type{
		fn from_file<F>($f: &mut F)->Result<
			Self,
			FromFileErr<$invalid_data,$other_type>
		> where
			Self: Sized,
			F: Read,
			F: Seek
		$body
	}
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

impl_from_file!(u32, (), (), f, {
	Ok((unwrap_or_ret!(u16::from_file(f)) as u32) << 16 | unwrap_or_ret!(u16::from_file(f)) as u32)
});
impl_from_file!(u16, (), (), f, {
	Ok((unwrap_or_ret!(u8::from_file(f)) as u16) << 8 | unwrap_or_ret!(u8::from_file(f)) as u16)
});
impl_from_file!(u8, (), (), f, {
	let mut buf = [0u8];
	match f.read(buf.as_mut_slice()){
		Ok(1) => Ok(buf[0]),
		Ok(0) => Err(FromFileErr::EOF),
		Ok(_) => unreachable!(),
		Err(e) => {
			panic!("{}",e);
		}
	}
});

fn array_from_file<F, T, I, O>(f: &mut F, count: usize)->Result<Box<[T]>, FromFileErr<I, O>> where
	F: Read,
	F: Seek,
	T: FromFile<I, O>,
{
	let mut rec = Vec::with_capacity(count);
	for _ in 0..count{rec.push(unwrap_or_ret!(T::from_file(f)))}
	Ok(rec.into())
}
#[derive(Debug)]
pub struct OTTF{
	pub table_directory: TableDirectory,
}
impl_from_file!(OTTF, (), (), f, {Ok(Self{
	table_directory: unwrap_or_ret!(TableDirectory::from_file(f)),
})});

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
impl_from_file!(TableDirectory, (), (), f, {
	let sfnt_version = SFNTVer::from_u32(unwrap_or_ret!(u32::from_file(f)));
	let num_tables = unwrap_or_ret!(u16::from_file(f));
	return Ok(Self{
		sfnt_version,
		num_tables,
		search_range: unwrap_or_ret!(u16::from_file(f)),
		entry_selector: unwrap_or_ret!(u16::from_file(f)),
		range_shift: unwrap_or_ret!(u16::from_file(f)),
		table_records: unwrap_or_ret!(array_from_file(f, num_tables as usize)),
	});
});

#[derive(Debug)]
pub struct TableRecord{
	///Table identifier.
	pub table_tag: Tag,
	///Checksum for this table.
	pub checksum: u32,
	///Offset from beginning of font file.
	pub offset: Offset32,
	///Length of this table.
	pub length: u32,
	cached_table: Option<Result<Box<Table>, FromFileErr<(),Box<[u8]>>>>,
}
impl_from_file!(TableRecord, (), (), f, {Ok(Self{
	table_tag: unwrap_or_ret!(Tag::from_file(f)),
	checksum: unwrap_or_ret!(u32::from_file(f)),
	offset: unwrap_or_ret!(Offset32::from_file(f)),
	length: unwrap_or_ret!(u32::from_file(f)),
	cached_table: None,
})});
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

///Array of four uint8s (length = 32 bits) used to identify a table, design-variation axis, script, language system, feature, or baseline
#[derive(Debug)]
pub struct Tag{
	pub data: String,
}
impl_from_file!(Tag, (), (), f, {
	let in_range = |chr: u8|{chr>=0x20&&chr<=0x7E};
	Ok(Self{data: String::from_iter([
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
		char::from(val_or_ret!(unwrap_or_ret!(u8::from_file(f)), in_range, ())),
	])})
});

type Offset32 = u32;
type Offset16 = u16;

#[derive(Debug)]
pub enum Table{
	Name(NameTable)
}

#[derive(Debug)]
pub struct NameTable{
	///Table version number 
	pub version: u16,
	///Number of name records.
	pub count: u16,
	///Offset to start of string storage (from start of table).
	pub storage_offset: Offset16,
	storage_absolute: u64,
	///The name records where count is the number of records.
	pub name_records: Box<[NameRecord]>,
	///Version (=1) Number of language-tag records.
	pub lang_tag_count: Option<u16>,
	///Version (=1) The language-tag records where langTagCount is the number of records.
	pub lang_tag_record: Option<Box<[LangTagRecord]>>,
}
impl_from_file!(NameTable, (), (), f, {
	let start = f.seek(SeekFrom::Current(0)).unwrap();
	let version = unwrap_or_ret!(u16::from_file(f));
	let count = unwrap_or_ret!(u16::from_file(f));
	let storage_offset = unwrap_or_ret!(Offset16::from_file(f));
	let name_records = unwrap_or_ret!(array_from_file(f, count as usize));
	let lang_tag_count = if version == 0{None}else{Some(unwrap_or_ret!(u16::from_file(f)))};
	return Ok(Self{
		version,
		count,
		storage_offset,
		storage_absolute: start + (storage_offset as u64),
		name_records,
		lang_tag_count,
		lang_tag_record: match lang_tag_count{
			Some(count)=>{Some(unwrap_or_ret!(array_from_file(f, count as usize)))},
			None=>None
		},
	});
});
#[derive(Debug)]
pub struct LangTagRecord{
	///Language-tag string length (in bytes)
	pub length: u16,
	///Language-tag string offset from start of storage area (in bytes).
	pub lang_tag_offset: Offset16,
}
impl_from_file!(LangTagRecord, (), (), f, {Ok(Self{
	length: unwrap_or_ret!(u16::from_file(f)),
	lang_tag_offset: unwrap_or_ret!(Offset16::from_file(f)),
})});
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
impl_from_file!(NameRecord, (), (), f, {Ok(Self{
	platform_id: unwrap_or_ret!(u16::from_file(f)),
	encoding_id: unwrap_or_ret!(u16::from_file(f)),
	language_id: unwrap_or_ret!(u16::from_file(f)),
	name_id: unwrap_or_ret!(u16::from_file(f)),
	length: unwrap_or_ret!(u16::from_file(f)),
	string_offset: unwrap_or_ret!(Offset16::from_file(f)),
})});

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