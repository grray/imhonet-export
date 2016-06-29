use rustc_serialize::json;
use std::io;
use libxml;

#[derive(Debug)]
pub enum Error {
	Simple(&'static str),
	General(String),
	IOError(io::Error),
	JsonError(json::ParserError),
	XmlParseError(libxml::parser::XmlParseError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<json::ParserError> for Error {
    fn from(err: json::ParserError) -> Error {
        Error::JsonError(err)
    }
}

impl From<libxml::parser::XmlParseError> for Error {
    fn from(err: libxml::parser::XmlParseError) -> Error {
        Error::XmlParseError(err)
    }
}

impl From<()> for Error {
    fn from(_: ()) -> Error {
        Error::Simple("Unknown error")
    }
}

