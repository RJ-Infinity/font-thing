use std::str::FromStr;

use proc_macro::{TokenStream, TokenTree, Delimiter};

fn err(err: &str)->TokenStream{format!(
	"compile_error!(\"{}\");",
	err
).parse().unwrap()}
macro_rules! single_token_macro {($name: ident, $token_type: pat, $body: block) => {
	#[proc_macro]
	pub fn $name(input: TokenStream) -> TokenStream {
		let mut iter = input.into_iter();
		let token = iter.next();
		if token.is_none() || iter.next().is_some(){return err("Incorect amount of tokens"); }
		match token.unwrap(){$token_type => $body, _ => err("incorect type"),}
	}
};}
single_token_macro!(tag_name_as_u32, proc_macro::TokenTree::Ident(v), {
	let mut rv = 0;
	for c in v.to_string().as_bytes(){rv = (rv<<8)|(*c as u32)}
	proc_macro::TokenTree::Literal(proc_macro::Literal::u32_suffixed(rv)).into()
});
single_token_macro!(ident_to_upper, proc_macro::TokenTree::Ident(v), {
	proc_macro::TokenTree::Literal(
		proc_macro::Literal::string(&v.to_string().to_uppercase())
	).into()
});
single_token_macro!(ident_to_lower, proc_macro::TokenTree::Ident(v), {
	proc_macro::TokenTree::Literal(
		proc_macro::Literal::string(&v.to_string().to_lowercase())
	).into()
});

// macro_rules! ident {($val: expr) => {
// 	TokenTree::Ident(Ident::new($val,Span::call_site()))
// };}
// macro_rules! punct {($val: expr, $spacing: ident) => {
// 	TokenTree::Punct(Punct::new($val,proc_macro::Spacing::$spacing)),
// };}
// macro_rules! unit {() => {TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::new()))};}
// macro_rules! group {($delimiter: ident, $val: expr) => {
// 	TokenTree::Group(Group::new(Delimiter::$delimiter, TokenStream::from_iter(vec!$val)))
// };}

// #[proc_macro_attribute]
// pub fn from_file_count(attr: TokenStream, item: TokenStream) -> TokenStream{
// 	println!("{:#?}\n{:#?}", attr, item);
// 	item
// }

#[proc_macro_derive(FromFile, attributes(from_file_count))]
pub fn derive_from_file(stream: TokenStream) -> TokenStream {
	// println!("{:#?}",stream);
	let mut streami = stream.into_iter();
	let mut struct_name = None;
	let mut struct_values = Vec::new();

	while let Some(token) = streami.next(){
		if let TokenTree::Ident(i) = token.clone(){if i.to_string() == "struct"{
			if let Some(TokenTree::Ident(i)) = streami.next(){
				struct_name = Some(i.to_string());
			}else{return err("expected struct name after `struct`");}
		}}
		if let TokenTree::Group(g) = token.clone(){if g.delimiter() == Delimiter::Brace{
			if struct_name.is_none(){return err("found struct fields before struct name");}
			streami = g.stream().into_iter();
			let mut last_token = None;
			let mut next_prop_count = None;
			while let Some(token) = streami.next(){
				if let TokenTree::Punct(p) = token.clone(){if p.as_char() == ':'{
					if last_token.is_none(){return err("expected a field name before a type");}
					if let Some(TokenTree::Ident(f_name)) = last_token{
						if let Some(TokenTree::Ident(mut f_type)) = streami.next(){
							if next_prop_count.is_some(){
								let not_box_err = err("items with a count must be a boxed array");

								if f_type.to_string() != "Box"{return not_box_err;}

								if let Some(TokenTree::Punct(p)) = streami.next(){
									if p.as_char() != '<' {return not_box_err;}
								}else{return not_box_err;}

								if let Some(TokenTree::Group(t)) = streami.next(){
									if t.delimiter() != Delimiter::Bracket {return not_box_err;}
									let mut t = t.stream().into_iter();
									if let Some(TokenTree::Ident(b_type)) = t.next(){
										f_type = b_type;
										if t.next().is_some(){return not_box_err;}
									}else{return not_box_err;}
								}else{return not_box_err;}

								if let Some(TokenTree::Punct(p)) = streami.next(){
									if p.as_char() != '>' {return not_box_err;}
								}else{return not_box_err;}
							}
							struct_values.push((next_prop_count, f_name.to_string(), f_type.to_string()));
							next_prop_count = None;
						}else{return err("expected type after field name then `:`");}
					}else{return err("expected a field name before a type");}
				}else if p.as_char() == '#' {if let Some(TokenTree::Group(attr)) = streami.next(){
					if attr.delimiter() == Delimiter::Bracket{
						let mut attr = attr.stream().into_iter();
						if let Some(TokenTree::Ident(attr_name)) = attr.next(){
							if attr_name.to_string() == "from_file_count"{
								let count_name = match attr.next(){
									Some(TokenTree::Group(count_name))=>count_name,
									_=>return err("Expected parenthasis after `from_file_count`"),
								};
								if count_name.delimiter() != Delimiter::Parenthesis
								{ return err("Expected parenthasis after `from_file_count`"); }

								let mut count_name_iter = count_name.stream().into_iter();
								let count_name = match count_name_iter.next(){
									Some(TokenTree::Ident(count_name))=>count_name.to_string(),
									_=>return err("Expected name of count attr as argument to `from_file_count`"),
								};
								if count_name_iter.next().is_some()
								{ return err("`from_file_count` attribute only accepts one argument"); }
								next_prop_count = Some(count_name);
							}
						}
					}
				}}}
				last_token = Some(token);
			}
			break;
		}}
	}

	let mut definitions = String::new();
	let mut init = String::new();
	for val in struct_values {
		definitions.push_str(&if let Some(count) = val.0{format!(
			"let {} = unwrap_or_ret!(array_from_file(f, {} as usize));", val.1, count
		)}else {format!(
			"let {} = unwrap_or_ret!({}::from_file(f));", val.1, val.2
		)});
		init.push_str(&format!("{},", val.1));
	}

	TokenStream::from_str(format!(r###"
		#[automatically_derived]
		impl FromFile<(),()> for {}{{
			fn from_file<F>(f: &mut F)->Result<
				Self,
				FromFileErr<(),()>
			> where
				Self: Sized,
				F: Read,
				F: Seek
			{{
				{}
				Ok(Self{{{}}})
			}}
		}}
	"###, struct_name.unwrap(), definitions, init).as_str()).unwrap()
}
