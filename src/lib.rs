#[macro_use] extern crate log;
#[macro_use] extern crate hyper;
extern crate rustc_serialize;
extern crate libxml;


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
	pub author_id: u64,
	pub year: u32,
}

impl Item {
	pub fn new (id: u64) -> Item {
		Item { id: id, title: String::new(), title_orig: String::new(), author_id: 0, year: 0 }
	}
}  

#[derive(PartialEq, Debug)]
pub struct Author {
	pub name: String,
	pub name_orig: String,
}

pub type AuthorHashMap = HashMap<u64, Author>;

impl Author {
	pub fn new () -> Author {
		Author { name: String::new(), name_orig: String::new()}
	}
}  

#[derive(PartialEq, Debug)]
pub struct Rate {
	pub rate: u8,
	pub item: Item
}

/// returns Rates vector for given imhonet username
pub fn get_user_rates(user: &str) -> Vec<Rate> {
    let mut rates = Vec::new();
	let mut current_page = format!("http://user.imhonet.ru/web.php?path=content/books/rates/&user_domain={}&domain=user", user); 
    
    // get rates page by page
	loop {
	    let xhr = load_imhonet_xhr(&current_page).unwrap();
	    match json::Json::from_str(&xhr) {
	    	Err(error) => warn!("Got invalid json: {:?}\nJson: {}", error, &xhr), 
	    	Ok(json) => {
			    match parse_rates(&json) {
			    	Err(error) => warn!("Error getting rates: {:?}", error),
			    	Ok((new_rates, next_page)) => {
			    		rates.extend(new_rates.into_iter());
			    		match next_page {
			    			Some(url) => current_page = url.to_owned(),
			    			None => break,
			    		}
			    	}, 
			    }
	    	}
	    };
	}
    
    for rate in &mut rates {
	    let mut item = &mut rate.item;
	    let item_page_load = load_imhonet_html(&format!("http://books.imhonet.ru/element/{}", &item.id));
	    match item_page_load {
	    	Ok(item_page) => parse_item(&item_page, &mut item),
	    	Err(error) => warn!("Error getting item {} data: {:?}", &item.id, &error),
	    }
    };
    return rates;
}

use std::collections::HashMap;
/// returns HashMap, key author id, value Author, for given rates vector
pub fn get_authors_for_rates(rates: &Vec<Rate>) -> AuthorHashMap {
	let mut authors: AuthorHashMap = HashMap::new();
	let ids = rates.iter().map(|r| r.item.author_id);
	for id in ids {
		if authors.contains_key(&id) { 
			continue;
		}
	    let page_load = load_imhonet_html(&format!("http://imhonet.ru/person/{}", id));
	    match page_load {
	    	Ok(page) => {
				authors.insert(id, parse_author(&page)); 
	    	}
	    	Err(error) => warn!("Error getting author #{} data: {:?}", id, &error),
	    }
	}
	return authors;
}


use std::io::Read;
use hyper::header::{Accept, Referer,  qitem};
use hyper::mime::{Mime, TopLevel, SubLevel};
fn load_imhonet_page(url: &str, headers: hyper::header::Headers) -> Result<String, Error> {
	info!("Loading {}", url);
	
	let client = hyper::client::Client::new();

	let mut resp = client.get(url)
		.headers(headers)
		.header(Referer("http://imhonet.ru/".to_owned()))
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
	headers.set(Accept(vec![qitem(Mime(TopLevel::Application, SubLevel::Json, vec![]))]));
	headers.set(XRequestedWith("XMLHttpRequest".to_owned()));
	return load_imhonet_page(url, headers);
}

fn load_imhonet_html(url: &str) -> Result<String, Error> {		
	return load_imhonet_page(url, hyper::header::Headers::new());
}


use rustc_serialize::json;
/// make Rate vector from rates json
fn parse_rates(json: &json::Json) -> Result<(Vec<Rate>, Option<&str>), Error> {
	let link_next = require!(json.find_path(&["user_rates", "link_next"]), Error::Simple("no user_rates:link_next key in json"));
	let next_page = link_next.as_string();
	
	let content_rates = require!(json.find_path(&["user_rates", "content_rated"]), Error::Simple("no user_rates:content_rated key in json"));
	let content_rates_arr = require!(content_rates.as_array(), Error::Simple("wrong user_rates:content_rated key in json"));
	
	let mut rates: Vec<Rate> = Vec::new();
	for content_rate in content_rates_arr {
		// now, if something wrong with concrete record, just discard it and write warning, don't return error 
		if let Some(rate_obj) = content_rate.as_object() {
			if let Some(rate_id_key) = rate_obj.get("object_id") {
				if let Some(rate_id) = rate_id_key.as_u64() {
					if let Some(rate_score_key) = rate_obj.get("rate") {
						if let Some(rate_score) = rate_score_key.as_u64() {
				   			let rate = Rate {
				   				item: Item::new(rate_id),
				   				rate: rate_score as u8
				   			};
				   			rates.push(rate);
				   			continue;
						}
					}
				}
			}
		}
		warn!("invalid content_rated record: {}", content_rate);
	}
	return Ok((rates, next_page));
}  

use libxml::parser::Parser;
use libxml::xpath;
use libxml::tree::{Node, Document};
/// returns xpath context and document for given html as &str
fn parse_html(html: &str) -> Result<(xpath::Context, Document), Error> {
	let parser = Parser::default();
  	let doc = try!(parser.parse_string(html));
	let context = try!(xpath::Context::new(&doc));
  	Ok((context, doc))
}

/// evaluates xpath and returns single Node
fn eval_xpath(context: &xpath::Context, xpath: &str) -> Option<Node> {
	// errors here caused by bad xpath, but it is static, so this unwrap is fine
	let result = context.evaluate(xpath).unwrap();
	
	let nodes = result.get_nodes_as_vec();
	
	if nodes.len() != 1 {
		warn!("Xpath {} returned {} results instead of 1", xpath, nodes.len());
	} 
	
	return nodes.get(0).cloned();	
} 

/// evaluates xpath and returns single node content
fn eval_xpath_get_content(context: &xpath::Context, xpath: &str) -> String {
	match eval_xpath(context, xpath) {
		None => String::new(),
		Some(node) => node.get_content(),
	}
}

/// evaluates xpath and returns single node property by given name
fn eval_xpath_get_property(context: &xpath::Context, xpath: &str, property: &str) -> String {
	match eval_xpath(context, xpath) {
		None => String::new(),
		Some(node) => node.get_property(property).unwrap_or(String::new()),
	}
}

fn parse_item(page: &str, item: &mut Item) {
	match parse_html(page) {
		Err(error) => warn!("can't parse html: {:?}. Html: {}", error, page),
		Ok((context, _doc)) => {
		    item.title = eval_xpath_get_content(&context, "//div[@class='m-elementprimary-txt']/h1//text()");   
			
		    item.title_orig = eval_xpath_get_content(&context, "//div[@class='m-elementprimary-txt']/div[@class='m-elementprimary-language']//text()");   
			
		    item.year = eval_xpath_get_content(&context, "//div[@class='m_row']//span[@class='m_value']//text()").parse::<u32>().unwrap_or(0);   
			
			let author_url = eval_xpath_get_property(&context, "//div[@class='m_row is-actors']//a[@class='m_value']", "href");
			let author_url_trimmed = author_url.trim_right_matches('/');
			if let Some(pos) = author_url_trimmed.rfind('/') {
				if let Ok(id) = author_url_trimmed[(pos+1)..].parse::<u64>() {
					item.author_id = id;
					return;
				} 
			};
		    warn!("can't get id from author url: {}", author_url);
		}
	}
}

fn parse_author(page: &str) -> Author {
	let (context, _doc) = parse_html(page).unwrap();
	
	Author {
    	name: eval_xpath_get_content(&context, "//div[@class='m-elementprimary-txt']/h1/text()"),   	
    	name_orig: eval_xpath_get_content(&context, "//div[@class='m-elementprimary-txt']/div[@class='m-elementprimary-language']/text()"),
	}   	
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::{parse_rates, parse_item, parse_author};  
    use super::{parse_html, eval_xpath_get_content, eval_xpath_get_property};  
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
    	let json_str = r#"
{
	"user_rates":{
		"content_rated":[
			{
				"object_id":19133,
				"rate":10,
				"rate_date":"1227019832"
			},
			{
				"object_id":1672,
				"rate":10,
				"rate_date":"1227019946"
			},
			{
				"object_id":171084,
				"rate":9,
				"rate_date":"1227020005"
			}
		],
		"link_next":null,
		"link_recommend":"http://books.imhonet.ru/recommend/"
	}
}
    	"#;
    	let should_be = vec![
    		Rate {
    			rate: 10,
    			item: Item::new(19133),
    		},
    		Rate {
    			rate: 10,
    			item: Item::new(1672),
    		},
    		Rate {
    			rate: 9,
    			item: Item::new(171084),
    		},
    	];
    	let json = json::Json::from_str(&json_str).unwrap();
    	let (parsed, next_page) = parse_rates(&json).unwrap();
        assert_eq!(parsed, should_be);
        assert_eq!(next_page, None);
    }
    
    #[test]
    fn eval_xpath() {
		let (context, _doc) = parse_html("<html><div class='foo'>bar</div></html>").unwrap();
		
		assert_eq!(eval_xpath_get_content(&context, "//div[@class='foo']//text()"), "bar");   	
		assert_eq!(eval_xpath_get_property(&context, "//div", "class"), "foo");   	
    }
    
    
    #[test]
    fn parse_item_updates_orig() {
    	setup();
		let html = r#"
			<!DOCTYPE html>
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
				<div class="m_row">
                    <span class="m_caption">Год выпуска: </span>
                    <span class="m_value">1989</span>
                </div>	            
	        </html>
        "#;   
		let mut item = Item::new(1672);
		let should_be = Item {
			id: 1672,
			title: "Стража! Стража!".to_owned(),
			title_orig: "Guards! Guards!".to_owned(),
			author_id: 154490,
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
				<div class="m_row">
                    <span class="m_caption">Год выпуска: </span>
                    <span class="m_value">1999</span>
                </div>	            
				<div class="m_row is-actors">
	            	<span class="m_caption">Автор: </span>
	                <span class="m_value-wrap">
	                	<a href="http://imhonet.ru/person/3/" rel="nofollow" class="m_value">Макс Фрай</a>
	               	</span>
	            </div>
	        </html>
        "#;   
		let mut item = Item::new(19133);
		let should_be = Item {
			id: 19133,
			title: "Лабиринты Ехо 23: Книга огненных страниц".to_owned(),
			title_orig: String::new(),
			author_id: 3,
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
		let should_be = Author{name: "Терри Пратчетт".to_owned(), name_orig: "Terry Pratchett".to_owned()};
		let author = parse_author(html);
		assert_eq!(author, should_be); 	
    }
    
}	