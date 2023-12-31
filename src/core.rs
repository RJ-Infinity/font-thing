use std::io::{Read, Seek, SeekFrom};

use macros::FromFile;

use crate::char_sets::{MacOsRoman, Utf16, CharSetStr, Utf8, Utf16BMPOnly};

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
	let mut buf = Vec::with_capacity(count);
	for _ in 0..count{buf.push(unwrap_or_ret!(T::from_file(f)))}
	Ok(buf.into())
}
#[derive(Debug,FromFile)]
pub struct OTTF{
	pub table_directory: TableDirectory,
}

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
impl_from_file!(SFNTVer, (), (), f, {Ok(SFNTVer::from_u32(unwrap_or_ret!(u32::from_file(f))))});

#[derive(Debug,FromFile)]
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
	#[from_file_count(num_tables)]
	pub table_records: Box<[TableRecord]>,
}

#[derive(Debug,FromFile)]
pub struct TableRecord{
	///Table identifier.
	pub table_tag: Tag,
	///Checksum for this table.
	pub checksum: u32,
	///Offset from beginning of font file.
	pub offset: Offset32,
	///Length of this table.
	pub length: u32,
}

macro_rules! get_table{($table: expr, $table_type: ty, $f: ident) => {
	Ok($table(match <$table_type>::from_file($f){
		Ok(v) => v,
		Err(e) => match e{
			FromFileErr::EOF => return Err(FromFileErr::EOF),
			FromFileErr::InvalidData(_) => return Err(FromFileErr::InvalidData(())),
			FromFileErr::Other(_) => unreachable!(),
		},
	}).into())
};}
impl TableRecord{
	pub fn get_table<T>(&mut self, f: &mut T)->Result<Table, FromFileErr<(),Box<[u8]>>> where T:Read, T:Seek{
		let cur = f.seek(SeekFrom::Current(0));
		if f.seek(SeekFrom::Start(self.offset as u64)).is_err()
		{return Err(FromFileErr::EOF);}

		let rv = match self.table_tag.data.as_str(){
			"DSIG" => get_table!(Table::DSIG, DSIGTable, f),
			"name" => get_table!(Table::Name, NameTable, f),
			_ => Err(FromFileErr::Other([].into())),
		};
		f.seek(SeekFrom::Start(cur.unwrap())).unwrap();
		return rv;
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
	Name(NameTable),
	DSIG(DSIGTable),
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
#[derive(Debug,FromFile)]
pub struct LangTagRecord{
	///Language-tag string length (in bytes)
	pub length: u16,
	///Language-tag string offset from start of storage area (in bytes).
	pub lang_tag_offset: Offset16,
}
#[derive(Debug,FromFile)]
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
macro_rules! decode_string {($char_set: ty, $string :expr) => {
		match CharSetStr::<$char_set>::from_bytes($string){
			Ok(s)=>Ok(s.to_string()),
			Err(_)=>Err("Invalid String".to_string()),
		}
	};}
impl NameRecord{
	pub fn get_string<F>(&self, f: &mut F, parent: &NameTable)->Result<Box<[u8]>,FromFileErr<(),()>> where F: Read, F: Seek{
		let pos = f.seek(SeekFrom::Current(0)).unwrap();
		if f.seek(SeekFrom::Start(parent.storage_absolute+self.string_offset as u64)).is_err()
		{return Err(FromFileErr::EOF);}
		let rv = array_from_file(f, self.length as usize);
		
		// this should never fail as we were at this position before running the function
		let _ = f.seek(SeekFrom::Start(pos));
		return rv;
	}
	
	pub fn translate_string(&self, string: Box<[u8]>)->Result<String,String>{
		match self.platform_id {
			0 => match self.encoding_id{ // Unicode
				0 => todo!("unicode 1.0 semantics"),
				1 => todo!("unicode 1.1 semantics"),
				2 => todo!("ISO/IEC 10646 semantics"),
				3 => decode_string!(Utf16BMPOnly, &string),// Unicode 2.0 and onwards semantics Unicode BMP only
				4 => decode_string!(Utf16, &string),// Unicode 2.0 and onwards semantics Unicode full repertoire
				x => Err(format!("{} is an invalid Encoding ID for platform id 0",x)),
			},
			1 => match self.encoding_id{ // Macintosh
				0 => decode_string!(MacOsRoman, &string), // Roman
				1 => todo!("Japanese"),
				2 => todo!("Chinese (Traditional)"),
				3 => todo!("Korean"),
				4 => todo!("Arabic"),
				5 => todo!("Hebrew"),
				6 => todo!("Greek"),
				7 => todo!("Russian"),
				8 => todo!("RSymbol"),
				9 => todo!("Devanagari"),
				10 => todo!("Gurmukhi"),
				11 => todo!("Gujarati"),
				12 => todo!("Odia"),
				13 => todo!("Bangla"),
				14 => todo!("Tamil"),
				15 => todo!("Telugu"),
				16 => todo!("Kannada"),
				17 => todo!("Malayalam"),
				18 => todo!("Sinhalese"),
				19 => todo!("Burmese"),
				20 => todo!("Khmer"),
				21 => todo!("Thai"),
				22 => todo!("Laotian"),
				23 => todo!("Georgian"),
				24 => todo!("Armenian"),
				25 => todo!("Chinese (Simplified)"),
				26 => todo!("Tibetan"),
				27 => todo!("Mongolian"),
				28 => todo!("Geez"),
				29 => todo!("Slavic"),
				30 => todo!("Vietnamese"),
				31 => todo!("Sindhi"),
				32 => todo!("Uninterpreted"),
				x => Err(format!("{} is an invalid Encoding ID for platform id 1",x)),
			},
			3 => match self.encoding_id{ // Windows
				0 => todo!("Symbol"),
				1 => decode_string!(Utf16BMPOnly, &string),// Unicode BMP
				2 => todo!("ShiftJIS"),
				3 => todo!("PRC"),
				4 => todo!("Big5"),
				5 => todo!("Wansung"),
				6 => todo!("Johab"),
				7 => todo!("Reserved"),
				8 => todo!("Reserved"),
				9 => todo!("Reserved"),
				10 => decode_string!(Utf8, &string),// Unicode full repertoire
				x => Err(format!("{} is an invalid Encoding ID for platform id 2",x)),
			},
			x => Err(format!("{} is an invalid Platform ID", x)),
		}
	}
}

#[derive(Debug,FromFile)]
pub struct DSIGTable{
	///Version number of the DSIG table (0x00000001)
	pub version: u32,
	///Number of signatures in the table
	pub num_signatures: u16,
	///permission flags
	/// Bit 0: cannot be resigned
	/// Bits 1-7: Reserved (Set to 0)
	pub flags: u16,
	///Array of signature records
	#[from_file_count(num_signatures)]
	pub signature_records: Box<[SignatureRecord]>,
}
#[derive(Debug,FromFile)]
pub struct SignatureRecord{
	///Format of the signature
	pub format: u32,
	///Length of signature in bytes
	pub length: u32,
	///Offset to the signature block from the beginning of the table
	pub signature_block_offset: Offset32
}

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
