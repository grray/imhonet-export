extern crate imhonet_export;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate csv;
extern crate rustc_serialize;
extern crate chrono;

use imhonet_export::{get_user_rates, get_authors_for_rates, Rate, AuthorHashMap};
use std::fs::File;

fn main() {
	env_logger::init().unwrap();
	let writer = File::create("export.csv").expect("Can't open file export.csv");
	let rates = get_user_rates("grray");
	let authors = get_authors_for_rates(&rates);
	goodreads_export(&rates, &authors, writer);
}

#[derive(RustcEncodable)]
struct Goodreads<'a> {
    title: &'a str,
    author: &'a str,
    isbn: &'a str,
    my_rating: u8,
    average_rating: u8,
    publisher: &'a str,
    binding: &'a str,
    year_published: u32,
    original_publication_year: u32,
    date_read: &'a str,
    date_added: &'a str,
    bookshelves: &'a str, 
    my_review: &'a str
}

fn goodreads_export<W>(rates: &Vec<Rate>, authors: &AuthorHashMap, writer: W) where W: std::io::Write {
	let mut csv = csv::Writer::from_writer(writer);
	for rate in rates {
		let item = &rate.item;
		let rate_date = rate.date.map_or(String::new(), |d| d.to_string());
    	let result = csv.encode(Goodreads{
    		title: &item.title_orig,
    		author: authors.get(&item.author_id).map_or("", |a| &a.name_orig),
    		isbn: "",	
    		my_rating: goodreads_rating(rate.rate),	
    		average_rating: 5,
    		publisher: "",	
    		binding: "",	
    		year_published: item.year,	
    		original_publication_year: item.year,	
    		date_read: &rate_date,	
    		date_added: &rate_date,	
    		bookshelves: "",	
    		my_review: "",	
    	});
    	if result.is_err() {
    		warn!("Can't encode record in csv: {:?}", result); 
    	}
	}
}

fn goodreads_rating(rate: u8) -> u8 {
	match rate {
		1...5 => 1,
		6 => 2,
		7...10 => std::cmp::max(1, rate - 5),
		_ => 0,
	}
}

#[cfg(test)]
mod tests {
    use super::goodreads_rating;
    
    #[test]
    fn test_goodreads_rating() {
    	assert_eq!(goodreads_rating(10), 5);
    	assert_eq!(goodreads_rating(7), 2);
    	assert_eq!(goodreads_rating(6), 2);
    	assert_eq!(goodreads_rating(5), 1);
    	assert_eq!(goodreads_rating(1), 1);
    	assert_eq!(goodreads_rating(0), 0);
    }
}
