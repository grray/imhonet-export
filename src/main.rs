extern crate imhonet_export;
extern crate env_logger;

use imhonet_export::get_user_rates;

fn main() {
	env_logger::init().unwrap();
	println!("{:?}", get_user_rates("grray"));	
}
