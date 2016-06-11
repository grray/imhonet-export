use rustc_serialize;
use std::io;

#[derive(Debug)]
pub enum Error {
	Simple(&'static str),
	General(String),
	IOError(io::Error),
	JsonError(rustc_serialize::json::ParserError)
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<rustc_serialize::json::ParserError> for Error {
    fn from(err: rustc_serialize::json::ParserError) -> Error {
        Error::JsonError(err)
    }
}

