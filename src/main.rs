extern crate curl;
extern crate rustc_serialize;
#[macro_use] extern crate log;
extern crate env_logger;

pub mod errors;
use errors::Error;

macro_rules! require {
    ($expr:expr, $error:expr) => (match $expr {
        Option::Some(val) => val,
        Option::None => {
            return Err(::std::convert::From::from($error))
        }
    })
}

fn main() {
	env_logger::init().unwrap();	
    let page = load_page("http://grray.imhonet.ru/web.php?path=content/books/rates/&user_domain=grray&domain=user");
    let export = parse_page(&page.unwrap());
}

use curl::http;
fn load_page(url: &str) -> Result<String, Error> {
	let handle = http::handle().get(url)
		.header("Accept", "application/json")
		.header("X-Requested-With", "XMLHttpRequest")
		.header("Referer", "http://grray.imhonet.ru/content/books/rates/");
		
	let resp = try!(handle.exec());
  	//println!("code={}; headers={:?}; body={:?}", resp.get_code(), resp.get_headers(), resp.get_body());
  	
  	return Ok(String::from_utf8_lossy(resp.get_body()).into_owned());
}

use rustc_serialize::json;
fn parse_page(json: &str) -> Result<Vec<String>, Error> {
	let json = try!(json::Json::from_str(&json));
	let json_obj = require!(json.as_object(), Error::General("json empty"));
	let user_rates = require!(json_obj.get("user_rates"), Error::General("no user_rates key in json"));
	let user_rates_obj = require!(user_rates.as_object(), Error::General("wrong user_rates key in json"));
	let content_rates = require!(user_rates_obj.get("content_rated"), Error::General("no user_rates:content_rated key in json"));
	let content_rates_arr = require!(content_rates.as_array(), Error::General("wrong user_rates:content_rated key in json"));
	
	let mut rates: Vec<String> = Vec::new();
	for content_rate in content_rates_arr {
		if let Some(rate_obj) = content_rate.as_object() {
			if let Some(rate_id) = rate_obj.get("object_id") {
				if let Some(rate_score) = rate_obj.get("rate") {
					let title = rate_obj.get("title").unwrap().as_string().unwrap().to_string();
					let rate = (rate_score.as_f64().unwrap()/10.0) as f32;
					let item = format!("{}, , , {}", title, rate);
		   			rates.push(rate);
		   			continue;
				}
			}
		}
		warn!("invalid content_rated record: {}", content_rate);
	}
	return Ok(rates);
	
}  

#[cfg(test)]
mod tests {
    use super::*;
    
    fn get_test_json() -> String {
    	r#"
{
	"user_rates":{
		"layout":[
			{
				"title":"Книги",
				"url":"http://grray.imhonet.ru/content/books/rates/",
				"count":"128",
				"code":"books"
			}
		],
		"content_rated":[
			{
				"object_id":19133,
				"title":"Лабиринты Ехо 23: Книга огненных страниц",
				"url":"http://books.imhonet.ru/element/19096/",
				"img":"http://std.imhonet.ru/element/de/93/de934b26b1454c2c4c798aca19a16999/labirinty-eho-23-kniga-ognennyh-stranic.jpg",
				"countries":[
					
				],
				"year":1999,
				"categories":[
					"Фантастика и фэнтези"
				],
				"rate":10,
				"rate_date":"1227019832",
				"rate_average":"8.14",
				"rate_amount":"7899",
				"opinions_amount":48
			},
			{
				"object_id":1672,
				"title":"Стража Стража!",
				"url":"http://books.imhonet.ru/element/1670/",
				"img":"http://st1.imhonet.ru/element/15/df/15df7d963324596b3d7848c81704aafd/strazha-strazha.jpg",
				"countries":[
					
				],
				"year":1989,
				"categories":[
					"Фантастика и фэнтези"
				],
				"rate":10,
				"rate_date":"1227019946",
				"rate_average":"8.22",
				"rate_amount":"4107",
				"opinions_amount":51
			},
			{
				"object_id":171084,
				"title":"День триффидов",
				"url":"http://books.imhonet.ru/element/170848/",
				"img":"http://std.imhonet.ru/element/d5/01/d50158237f618b0b51253ff7797496e8/den-triffidov.jpg",
				"countries":[
					
				],
				"year":1951,
				"categories":[
					"Фантастика и фэнтези"
				],
				"rate":9,
				"rate_date":"1227020005",
				"rate_average":"8.01",
				"rate_amount":"4630",
				"opinions_amount":93
			}
		],
		"link_next":"http://grray.imhonet.ru/content/books/rates/?sort_by=asc&page=2",
		"link_recommend":"http://books.imhonet.ru/recommend/"
	}
}
    	"#.to_string()
    }

    #[test]
    fn parse_response() {
    	let parsed = user.parse_response(&get_test_json());
    	let should_be = Ok(vec![
    		Rate {
    			rate: 1.0,
    			item: Item {
    				id: 19133,
    				title: "Лабиринты Ехо 23: Книга огненных страниц".to_string(),
    				author: "".to_string(),
    			}
    		},
    		Rate {
    			rate: 1.0,
    			item: Item {
    				id: 1672,
    				title: "Стража Стража!".to_string(),
    				author: "".to_string(),
    			}
    		},
    		Rate {
    			rate: 0.9,
    			item: Item {
    				id: 171084,
    				title: "День триффидов".to_string(),
    				author: "".to_string(),
    			}
    		},
    	]);
        assert_eq!(parsed, should_be);
    }
}	