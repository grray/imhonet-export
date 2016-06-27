extern crate imhonet_export;
extern crate env_logger;

use imhonet_export::{get_user_rates, get_authors_for_rates};

fn main() {
	env_logger::init().unwrap();
	let rates = get_user_rates("grray");
	let authors = get_authors_for_rates(&rates);
	println!("rates: {:?}", rates);	
	println!("authors: {:?}", authors);	
}
