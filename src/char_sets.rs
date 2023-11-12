use std::{char, collections::TryReserveError, vec::Drain, ops::{RangeBounds, Add, AddAssign, Index, Range}, fmt, cmp::min};


pub trait CharSetChar: Clone + PartialEq{
	/// Consumes bytes from the front of the vec until an error occurs or a character is formed. If an error occurs no bytes are consumed.
	fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized;
	/// Converts the bytearray into a single character. Fails if the bytes are not valid, too few or too numerous
	fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized;
	/// Converts the CharSet into a Vec of Bytes that should be the reverse of from_bytes (i.e. if you put the output of get_bytes into a slice and then into from_bytes it should return the same character you started with)
	/// ```
	/// use font::char_sets::{CharSetChar,Ascii};
	/// let c: Ascii = CharSetChar::from_native('a').unwrap(); // any valid character
	/// assert_eq!(CharSetChar::from_bytes(&c.get_bytes()), Ok(c));
	/// ```
	fn get_bytes(&self) -> Vec<u8>;
	/// Converts the CharSet into a native rust char
	fn as_native(&self) -> char;
	/// reverse of `as_native`
	fn from_native(chr: char) -> Result<Self, ()>where Self: Sized;
}

macro_rules! shadow_constructor {
	($fn: ident) => {pub fn $fn()->Self{Self{data:Vec::$fn()}}};
	($fn: ident, $arg0: ty ) => {
		pub fn $fn(arg0: $arg0)->Self{Self{data:Vec::$fn(arg0)}}
	};
}
macro_rules! shadow {
	($fn: ident, $rv: ty) => {pub fn $fn(&self)->$rv{self.data.$fn()}};
	($fn: ident, $arg0: ty, $rv: ty) => {
		pub fn $fn(&self, arg0: $arg0)->$rv{self.data.$fn(arg0)}
	};
}
macro_rules! shadow_mut {
	($fn: ident, $rv: ty) => {pub fn $fn(&mut self)->$rv{self.data.$fn()}};
	($fn: ident, $arg0: ty, $rv: ty) => {
		pub fn $fn(&mut self, arg0: $arg0)->$rv{self.data.$fn(arg0)}
	};
	($fn: ident, $arg0: ty, $arg1: ty, $rv: ty) => {
		pub fn $fn(&mut self, arg0: $arg0, arg1: $arg1)->$rv{self.data.$fn(arg0, arg1)}
	};
}

/// This is a string using a particular char set
/// ```
/// use font::char_sets::CharSetStr;
/// let s = CharSetStr::<font::char_sets::CodePage437>::from_bytes(&[1u8,2u8,3u8]).unwrap();
/// assert_eq!(s.to_string(),"☺☻♥");
/// ```
#[derive(Clone, PartialEq, Default, Hash)]
pub struct CharSetStr<T> where T: CharSetChar{data: Vec<T>}
impl<T> CharSetStr<T> where T: CharSetChar{
	pub fn from_char_set_chars(data: Vec<T>)->Self{Self{data}}
	pub fn from_bytes(bytes: &[u8])->Result<Self, usize>{
		let mut m_bytes = bytes.to_vec();
		let mut rv = Self{data: vec!()};
		while let Ok(c) = T::consume_bytes(&mut m_bytes) {
			rv.data.push(c);
			if m_bytes.len() == 0{ return Ok(rv); }
		}
		Err(bytes.len() - m_bytes.len())
	}
	pub fn to_bytes(&self)->Vec<u8>
	{ self.data.iter().map(|c|c.get_bytes()).flatten().collect() }

	pub fn from_string(string: &str)->Result<Self, usize>{
		let mut rv = Self{data: vec!()};

		for (i, chr) in string.char_indices()
		{ if let Ok(c) = T::from_native(chr){ rv.data.push(c); }else{ return Err(i); } }

		Ok(rv)
	}
	pub fn to_string(&self)->String
	{ self.data.iter().map(|chr|chr.as_native()).collect() }

	pub fn to_char_set_str<U>(&self)->Result<CharSetStr<U>, ()> where U: CharSetChar{
		let rv = CharSetStr::from_string(&self.to_string());
		if rv.is_err(){Err(())}else{Ok(rv.unwrap())}
	}
	
	pub const fn new()->Self{Self{data:vec![]}} // not a shadow because of the const
	shadow_constructor!(with_capacity, usize);
	shadow!(capacity, usize);
	shadow_mut!(reserve, usize, ());
	shadow_mut!(reserve_exact, usize, ());
	shadow_mut!(try_reserve, usize, Result<(), TryReserveError>);
	shadow_mut!(try_reserve_exact, usize, Result<(), TryReserveError>);
	shadow_mut!(shrink_to_fit, ());
	shadow_mut!(shrink_to, usize, ());
	shadow_mut!(push, T, ());
	shadow_mut!(truncate, usize, ());
	shadow_mut!(pop, Option<T>);
	shadow_mut!(remove, usize, T);
	pub fn retain<F>(&mut self, f: F)where F: FnMut(&T) -> bool{self.data.retain(f)}
	shadow_mut!(insert, usize, T, ());
	shadow!(len, usize);
	shadow!(is_empty, bool);
	pub fn split_off(&mut self, at: usize)->Self { Self::from_char_set_chars(self.data.split_off(at)) }
	shadow_mut!(clear, ());
	pub fn drain<R>(&mut self, range: R) -> Drain<'_, T> where R: RangeBounds<usize>{self.data.drain(range)}
}
impl<T> Add<&CharSetStr<T>> for CharSetStr<T> where T: CharSetChar{
	type Output = CharSetStr<T>;

	fn add(self, rhs: &CharSetStr<T>) -> Self::Output {
		let mut data = Vec::with_capacity(self.len()+rhs.len());
		data.append(&mut self.data.clone());
		data.append(&mut rhs.data.clone());
		Self::Output{data}
	}
}
impl<T> AddAssign<&CharSetStr<T>> for CharSetStr<T> where T: CharSetChar{
	fn add_assign(&mut self, rhs: &CharSetStr<T>)
	{ self.data.append(&mut rhs.data.clone()); }
}
impl<T> fmt::Debug for CharSetStr<T> where T: CharSetChar{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
		f.debug_struct(format!("CharSetStr<{}>",std::any::type_name::<T>()).as_str())
		.field("data", &self.to_bytes())
		.field("native", &self.to_string())
		.finish()
	}
}
impl<T> std::fmt::Display for CharSetStr<T> where T: CharSetChar{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{ f.write_str(&self.to_string()) }
}
impl<T> Extend<T> for CharSetStr<T> where T: CharSetChar
{ fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {self.data.extend(iter)} }
impl<T> PartialOrd for CharSetStr<T> where T: CharSetChar{
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
	{ self.to_bytes().partial_cmp(&other.to_bytes()) }
}
// TODO: impl ord onwards

#[derive(Clone, PartialEq)]
pub struct Utf8{data:char}
impl CharSetChar for Utf8{
	fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized {
		if bytes.len() < 1{return Err(())};
		let s = match String::from_utf8(bytes[..min(bytes.len(), 4)].to_vec()){
			Ok(s)=>s,
			Err(e)=>{
				let i = e.utf8_error().valid_up_to();
				if i == 0{ return Err(()); }
				String::from_utf8(bytes[..i].to_vec()).unwrap()
			},
		};
		let chr = s.chars().next().unwrap();
		for _ in 0..chr.len_utf8(){bytes.remove(0);}
		Ok(Self{data:chr})
	}
	fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized {
		let string = match String::from_utf8(bytes.to_vec()){Ok(s)=>s,Err(_)=>return Err(())};
		if string.chars().count() != 1{return Err(());}
		Ok(Self{data:string.chars().next().unwrap()})
	}
	fn get_bytes(&self) -> Vec<u8> {
		let mut buf = [0; 4];
		self.data.encode_utf8(&mut buf);
		buf[..self.data.len_utf8()].to_vec()
	}
	fn as_native(&self) -> char{self.data}
	fn from_native(chr: char) -> Result<Self, ()>where Self: Sized {Ok(Self{data: chr})}
}

#[derive(Clone, PartialEq)]
pub struct Utf16{data:char}
impl CharSetChar for Utf16{
	fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized {
		if bytes.len() < 2{return Err(())};
		let mut buf = if bytes.len() < 4{vec![(bytes[0]as u16)<<8 | bytes[1]as u16]}
		else{vec![(bytes[0]as u16)<<8 | bytes[1]as u16, (bytes[2]as u16)<<8 | bytes[3]as u16]};

		let s = match String::from_utf16(&buf){
			Ok(s)=>s,
			Err(_)=>{if buf.len() == 2{
				buf.pop();
				match String::from_utf16(&buf){
					Ok(s)=>s,
					Err(_)=>return Err(()),
				}
			}else{ return Err(()); }},
		};
		let chr = s.chars().next().unwrap();
		for _ in 0..chr.len_utf16()*2{bytes.remove(0);}
		Ok(Self{data:chr})
	}
	fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized {
		let string = match String::from_utf16(&bytes.chunks(2).map(|c|{
			(c[0]as u16)<<8 | c[1]as u16
		}).collect::<Vec<u16>>()){Ok(s)=>s,Err(_)=>return Err(())};
		if string.chars().count() != 1{return Err(());}
		Ok(Self{data:string.chars().next().unwrap()})
	}
	fn get_bytes(&self) -> Vec<u8> {
		let mut buf = [0; 2];
		self.data.encode_utf16(&mut buf);
		buf[..self.data.len_utf16()].iter().flat_map(|c|{[
			(c>>8) as u8,
			*c as u8,
		]}).collect()
	}
	fn as_native(&self) -> char{self.data}
	fn from_native(chr: char) -> Result<Self, ()>where Self: Sized {Ok(Self{data: chr})}
}

#[derive(Clone, PartialEq)]
pub struct Utf16BMPOnly{data:u16}
impl CharSetChar for Utf16BMPOnly{
	fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized {
		if bytes.len() < 2{return Err(())};

		let buf = vec![(bytes[0]as u16)<<8 | bytes[1]as u16];

		if String::from_utf16(&buf).is_err(){return Err(()); };

		bytes.remove(0);
		bytes.remove(0);
		
		Ok(Self{data:buf[0]})
	}
	fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized {
		if bytes.len() != 2{return Err(());}
		let data = (bytes[0]as u16)<<8 | bytes[1]as u16;
		match String::from_utf16(&[data]){Ok(s)=>s,Err(_)=>return Err(())};
		Ok(Self{data})
	}
	fn get_bytes(&self) -> Vec<u8> {vec![(self.data>>8) as u8, self.data as u8]}
	fn as_native(&self) -> char{String::from_utf16(&[self.data]).unwrap().chars().next().unwrap()}
	fn from_native(chr: char) -> Result<Self, ()>where Self: Sized {
		if chr.len_utf16() != 1{Err(())}else{
			let mut buf = [0u16];
			chr.encode_utf16(&mut buf);
			Ok(Self{data: buf[0]})
		}
	}
}

#[derive(Clone, PartialEq)]
pub struct Ascii{data: u8}
impl CharSetChar for Ascii{
	fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized {
		if bytes.len() < 1 || bytes[0] >= 0x80{ Err(()) }
		else{ Ok(Self{data:bytes.remove(0)})}
	}
	
	fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized {
		if bytes.len() != 1 || bytes[0] >= 0x80{ Err(()) }
		else{ Ok(Self{data: bytes[0]}) }
	}
	
	fn get_bytes(&self) -> Vec<u8> {vec![self.data]}
	
	fn as_native(&self) -> char
	{ String::from_utf8(self.get_bytes()).unwrap().chars().next().unwrap() }

	fn from_native(chr: char) -> Result<Self, ()>where Self: Sized {
		if chr.len_utf8() == 1{
			let mut buf = [0; 1];
			chr.encode_utf8(&mut buf);
			if buf[0] > 0x80 { Err(()) }else{ Ok(Self{data: buf[0]}) }
		}else{Err(())}
	}
}
impl fmt::Debug for Ascii{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{ f.debug_struct("Ascii").field("data", &self.as_native()).finish() }
}

macro_rules! define_byte_char_set {($name: ident, $chars: expr, $inval: expr) => {
	#[derive(Clone, PartialEq)]
	pub struct $name{data: u8}
	impl $name{
		fn new(data: u8)->Result<Self, ()>{
			if data < 0x80 { Ok(Self{data}) }
			else if Self::CHARS[(data-0x80) as usize] == $inval{ Err(()) }
			else { Ok(Self{data}) }
		}
		const CHARS: [char; 256] = $chars;
	}
	impl CharSetChar for $name{
		fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized
		{ if bytes.len() < 1{ Err(()) }else{ Self::new(bytes.remove(0)) } }
		
		fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized
		{ if bytes.len() != 1{ Err(()) }else{ Self::new(bytes[0]) } }
		
		fn get_bytes(&self) -> Vec<u8> {vec![self.data]}
		
		fn as_native(&self) -> char{Self::CHARS[self.data as usize]}
		fn from_native(chr: char) -> Result<Self, ()>where Self: Sized {
			for (i, val) in Self::CHARS.iter().enumerate()
			{ if chr == *val{return Self::new(i as u8)} }
			Err(())
		}
	}
	impl fmt::Debug for $name{
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
		{
			f.debug_struct(stringify!($name))
			.field("data", &self.data)
			.field("as_native", &self.as_native())
			.finish()
		}
	}
};}

macro_rules! define_extended_ascii {($name: ident, $chars: expr, $inval: expr) => {
	#[derive(Clone, PartialEq)]
	pub struct $name{data: u8}
	impl $name{
		fn new(data: u8)->Result<Self, ()>{
			if data < 0x80 { Ok(Self{data}) }
			else if Self::CHARS[(data-0x80) as usize] == $inval{ Err(()) }
			else { Ok(Self{data}) }
		}
		const CHARS: [char; 128] = $chars;
	}
	impl CharSetChar for $name{
		fn consume_bytes(bytes: &mut Vec<u8>) -> Result<Self, ()>where Self: Sized
		{ if bytes.len() < 1{ Err(()) }else{ Self::new(bytes.remove(0)) } }
		
		fn from_bytes(bytes: &[u8]) -> Result<Self, ()>where Self: Sized
		{ if bytes.len() != 1{ Err(()) }else{ Self::new(bytes[0]) } }
		
		fn get_bytes(&self) -> Vec<u8> {vec![self.data]}
		
		fn as_native(&self) -> char{
			if self.data < 0x80 {self.data as char}
			else{Self::CHARS[(self.data-0x80) as usize]}
		}
		fn from_native(chr: char) -> Result<Self, ()>where Self: Sized {
			if let Ok(chr) = Ascii::from_native(chr){
				Ok(Self{data: chr.get_bytes()[0]})
			}else{
				for (i, val) in Self::CHARS.iter().enumerate()
				{ if chr == *val{return Self::new(0x80 + (i as u8))} }
				Err(())
			}
		}
	}
	impl fmt::Debug for $name{
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
		{
			f.debug_struct(stringify!($name))
			.field("data", &self.data)
			.field("as_native", &self.as_native())
			.finish()
		}
	}
};}

define_byte_char_set!{CodePage437, [
	'\0', '☺', '☻', '♥', '♦', '♣', '♠', '•', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼',
	'►', '◄', '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼',
	' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
	'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
	'@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
	'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
	'`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
	'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '⌂',
	'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
	'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
	'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
	'░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
	'└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
	'╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
	'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
	'≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{202F}',
], '\u{100}'}

define_extended_ascii!{MacOsRoman, [
	'Ä', 'Å', 'Ç', 'É', 'Ñ', 'Ö', 'Ü', 'á', 'à', 'â', 'ä', 'ã', 'å', 'ç', 'é', 'è',
	'ê', 'ë', 'í', 'ì', 'î', 'ï', 'ñ', 'ó', 'ò', 'ô', 'ö', 'õ', 'ú', 'ù', 'û', 'ü',
	'†', '°', '¢', '£', '§', '•', '¶', 'ß', '®', '©', '™', '´', '¨', '≠', 'Æ', 'Ø',
	'∞', '±', '≤', '≥', '¥', 'µ', '∂', '∑', '∏', 'π', '∫', 'ª', 'º', 'Ω', 'æ', 'ø',
	'¿', '¡', '¬', '√', 'ƒ', '≈', '∆', '«', '»', '…', '\u{202F}', 'À', 'Ã', 'Õ', 'Œ', 'œ',
	'–', '—', '“', '”', '‘', '’', '÷', '◊', 'ÿ', 'Ÿ', '⁄', '€', '‹', '›', 'ﬁ', 'ﬂ',
	'‡', '·', '‚', '„', '‰', 'Â', 'Ê', 'Á', 'Ë', 'È', 'Í', 'Î', 'Ï', 'Ì', 'Ó', 'Ô',
	'\u{F8FF}', 'Ò', 'Ú', 'Û', 'Ù', 'ı', 'ˆ', '˜', '¯', '˘', '˙', '˚', '¸', '˝', '˛', 'ˇ',
], '\0'}

