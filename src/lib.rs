#[macro_use] extern crate log;
#[macro_use] extern crate hyper;
extern crate rustc_serialize;
extern crate sxd_xpath;
extern crate sxd_document;


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


#[derive(PartialEq, Debug)]
pub struct Item {
	pub id: u64,
	pub title: String,
	pub title_orig: String,
	pub author: Author,		
	pub year: u32,
}

impl Item {
	pub fn new () -> Item {
		Item { id: 0, title: String::new(), title_orig: String::new(), author: Author::new(), year: 0 }
	}
}  

#[derive(PartialEq, Debug)]
pub struct Author {
	pub id: u64,
	pub name: String,
	pub name_orig: String,
}

impl Author {
	pub fn new () -> Author {
		Author { id: 0, name: String::new(), name_orig: String::new()}
	}
}  


#[derive(PartialEq, Debug)]
pub struct Rate {
	pub rate: u8,
	pub item: Item
}

use std::io::Read;
use hyper::header::{Accept, Referer,  qitem};
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};

fn load_imhonet_page(url: &str, headers: hyper::header::Headers) -> Result<String, Error> {
	let client = hyper::client::Client::new();
	
	let mut resp = client.get(url)
		.header(Accept(vec![qitem(Mime(TopLevel::Application, SubLevel::Json, vec![(Attr::Charset, Value::Utf8)]))]))
		.header(Referer("http://imhonet.ru/".to_owned()))  //TODO: remove and test
		.headers(headers)
		.send().unwrap();
	
	if resp.status != hyper::Ok {
		return Err(Error::General(format!("Can't load {}, got error {:?}", url, resp.status)));
	}
	
	let mut body = String::new();
	
	try!(resp.read_to_string(&mut body));  	
  	
  	return Ok(body);	
}

header! { (XRequestedWith, "X-Requested-With") => [String] }
fn load_imhonet_xhr(url: &str) -> Result<String, Error> {	
	let mut headers = hyper::header::Headers::new();
	headers.set(XRequestedWith("XMLHttpRequest".to_owned()));
	return load_imhonet_page(url, headers);
}

fn load_imhonet_html(url: &str) -> Result<String, Error> {		
	return load_imhonet_page(url, hyper::header::Headers::new());
}

use rustc_serialize::json;
fn parse_rates(json: &json::Json) -> Result<Vec<Rate>, Error> {
	let content_rates = require!(json.find_path(&["user_rates", "content_rated"]), Error::Simple("no user_rates:content_rated key in json"));
	let content_rates_arr = require!(content_rates.as_array(), Error::Simple("wrong user_rates:content_rated key in json"));
	
	let mut rates: Vec<Rate> = Vec::new();
	for content_rate in content_rates_arr {
		// now, if something wrong with concrete record, just discard it and write warning, don't return error 
		if let Some(rate_obj) = content_rate.as_object() {
			if let Some(rate_id) = rate_obj.get("object_id") {
				if let Some(rate_score) = rate_obj.get("rate") {
					if let Some(title) = rate_obj.get("title") {
			   			let item = Item {
			   				id: rate_id.as_u64().unwrap(),
			   				title: title.as_string().unwrap_or("").to_owned(),
			   				title_orig: String::new(),
			   				author: Author::new(),
			   				year: rate_obj.get("year").map(|y| y.as_u64().unwrap_or(0) as u32).unwrap_or(0),
			   			};				
			   			let rate = Rate {
			   				item: item,
			   				rate: rate_score.as_u64().unwrap() as u8
			   			};
			   			rates.push(rate);
			   			continue;
					}
				}
			}
		}
		warn!("invalid content_rated record: {}", content_rate);
	}
	return Ok(rates);
}  

fn get_package(html: &str) -> Result<sxd_document::Package, (usize, Vec<sxd_document::parser::Error>)> {
    return sxd_document::parser::parse(html)
}

use std::collections::HashMap;
fn eval_xpath(package: &sxd_document::Package, xpath: &str) -> String {
	// errors here caused by bad xpath, but it is static, so these unwrap's are fine
	let expression = sxd_xpath::Factory::new().build(xpath).unwrap().unwrap();
	let root = package.as_document().root();
	let mut functions = HashMap::new(); 
	sxd_xpath::function::register_core_functions(&mut functions);
	let variables = HashMap::new(); 
	let namespaces = HashMap::new(); 
    let context = sxd_xpath::EvaluationContext::new(root, &functions, &variables, &namespaces);
	
    if let sxd_xpath::Value::String(result) = expression.evaluate(&context).unwrap() {
    	return result.to_owned();
    } else {
	    warn!("not a string returned by xpath {}", xpath);
		return String::new();
    }
} 

fn parse_item(page: &str, item: &mut Item) {
	let package = get_package(page).unwrap();
	
	let title_orig = eval_xpath(&package, "string(//div[@class='m-elementprimary-txt']/div[@class='m-elementprimary-language'])");
    item.title_orig = title_orig;   
	
	let author_name = eval_xpath(&package, "string(//div[@class='m_row is-actors']//a[@class='m_value']/text())");
    item.author.name = author_name;
    
	let author_url = eval_xpath(&package, "string(//div[@class='m_row is-actors']//a[@class='m_value']/@href)");
	let author_url_trimmed = author_url.trim_right_matches('/');
	if let Some(pos) = author_url_trimmed.rfind('/') {
		if let Ok(id) = author_url_trimmed[(pos+1)..].parse::<u64>() {
			item.author.id = id;
			return;
		} 
	};
    warn!("can't get id from author url: {}", author_url);
}

fn parse_author(page: &str, author: &mut Author) {
	let package = get_package(page).unwrap();
	
	let name_orig = eval_xpath(&package, "string(//div[@class='m-elementprimary-txt']/div[@class='m-elementprimary-language'])");
    author.name_orig = name_orig;   	
}

pub fn get_user_rates(user: &str) -> Vec<Rate> {
	let url = format!("http://user.imhonet.ru/web.php?path=content/books/rates/&user_domain={}&domain=user", user); 
    let xhr = load_imhonet_xhr(&url).unwrap();
    let json = json::Json::from_str(&xhr).unwrap();
    let mut rates = parse_rates(&json).unwrap();
    for rate in &mut rates {
	    let mut item = &mut rate.item;
	    let item_page_load = load_imhonet_html(&format!("http://imhonet.ru/person/{}", &item.id));
	    match item_page_load {
	    	Ok(item_page) => parse_item(&item_page, &mut item),
	    	Err(error) => warn!("Error getting item {} data: {:?}", &item.id, &error),
	    }
    };
    return rates;
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::{parse_rates, parse_item, parse_author};  
	extern crate env_logger;
	
	use std::sync::{Once, ONCE_INIT};

	static SETUP: Once = ONCE_INIT;
	    
    fn setup() {
		SETUP.call_once(|| {
		    env_logger::init().unwrap();
		});
    }
    
    use rustc_serialize::json;
    #[test]
    fn parse_rates_return_rates() {
    	setup();
    	let json = r#"
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
				"countries":[],
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
				"countries":[],
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
				"countries":[],
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
		"link_next":"http://user.imhonet.ru/content/books/rates/?sort_by=asc&page=2",
		"link_recommend":"http://books.imhonet.ru/recommend/"
	}
}
    	"#;
    	let should_be = vec![
    		Rate {
    			rate: 10,
    			item: Item {
    				id: 19133,
    				title: "Лабиринты Ехо 23: Книга огненных страниц".to_owned(),
    				title_orig: String::new(),
    				author: Author::new(),
    				year: 1999,
    			}
    		},
    		Rate {
    			rate: 10,
    			item: Item {
    				id: 1672,
    				title: "Стража Стража!".to_owned(),
    				title_orig: String::new(),
    				author: Author::new(),
    				year: 1989,
    			}
    		},
    		Rate {
    			rate: 9,
    			item: Item {
    				id: 171084,
    				title: "День триффидов".to_owned(),
    				title_orig: String::new(),
    				author: Author::new(),
    				year: 1951,
    			}
    		},
    	];
    	let parsed = parse_rates(&json::Json::from_str(&json).unwrap()).unwrap();
        assert_eq!(parsed, should_be);
    }
    
    #[test]
    fn parse_item_updates_orig() {
    	setup();
		let html = r#"
			<html>
				<div class="m-elementprimary-txt">
	            	<h1 class="m-elementprimary-title">Стража! Стража!</h1>
	                <div class="m-elementprimary-language">Guards! Guards!</div>
	            </div>
				<div class="m_row is-actors">
	            	<span class="m_caption">Автор: </span>
	                <span class="m_value-wrap">
	                	<a href="http://imhonet.ru/person/154490/" rel="nofollow" class="m_value">Терри Пратчетт</a>
	               	</span>
	            </div>
	        </html>
        "#;   
		let mut item = Item {
			id: 1672,
			title: "Стража Стража!".to_owned(),
			title_orig: String::new(),
			author: Author::new(),
			year: 1989,
		};
		let should_be = Item {
			id: 1672,
			title: "Стража Стража!".to_owned(),
			title_orig: "Guards! Guards!".to_owned(),
			author: Author{id: 154490, name: "Терри Пратчетт".to_owned(), name_orig: String::new()},
			year: 1989,
		};
		parse_item(html, &mut item);
		assert_eq!(item, should_be); 	
    }
    
    #[test]
    fn parse_item_without_orig() {
    	setup();
		let html = r#"
			<html>
				<div class="m-elementprimary-txt">
	            	<h1 class="m-elementprimary-title">Лабиринты Ехо 23: Книга огненных страниц</h1>
	                <div class="m-elementprimary-language"></div>
	            </div>
				<div class="m_row is-actors">
	            	<span class="m_caption">Автор: </span>
	                <span class="m_value-wrap">
	                	<a href="http://imhonet.ru/person/3/" rel="nofollow" class="m_value">Макс Фрай</a>
	               	</span>
	            </div>
	        </html>
        "#;   
    	let mut item = Item {
			id: 19133,
			title: "Лабиринты Ехо 23: Книга огненных страниц".to_owned(),
			title_orig: String::new(),
			author: Author::new(),
			year: 1999,
		};
		let should_be = Item {
			id: 19133,
			title: "Лабиринты Ехо 23: Книга огненных страниц".to_owned(),
			title_orig: String::new(),
			author: Author{id: 3, name: "Макс Фрай".to_owned(), name_orig: String::new()},
			year: 1999,
		};
		parse_item(html, &mut item);
		assert_eq!(item, should_be); 	
    }
    
    #[test]
    fn parse_author_updates_orig() {
    	setup();
		let html = r#"
			<html>
				<div class="m-elementprimary-txt">
	            	<h1 class="m-elementprimary-title">Терри Пратчетт</h1>
	                <div class="m-elementprimary-language">Terry Pratchett</div>
	            </div>
	        </html>
        "#;   
		let mut author = Author{id: 154490, name: "Терри Пратчетт".to_owned(), name_orig: String::new()};
		let should_be = Author{id: 154490, name: "Терри Пратчетт".to_owned(), name_orig: "Terry Pratchett".to_owned()};
		parse_author(html, &mut author);
		assert_eq!(author, should_be); 	
    }
    
}	