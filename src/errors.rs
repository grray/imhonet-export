use curl;
use rustc_serialize;

#[derive(Debug)]
enum Error {
	General(String),
	CurlError(curl::ffi::err::ErrCode),
	JsonError(rustc_serialize::json::ParserError)
}

impl From<curl::ffi::err::ErrCode> for Error {
    fn from(err: curl::ffi::err::ErrCode) -> Error {
        Error::CurlError(err)
    }
}

impl From<rustc_serialize::json::ParserError> for Error {
    fn from(err: rustc_serialize::json::ParserError) -> Error {
        Error::JsonError(err)
    }
}

