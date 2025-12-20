use http::{header::*, response::Builder as ResponseBuilder, status::StatusCode};
use crate::shared::config_dir;

pub fn iconpack_access_protocol(request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, Box<dyn std::error::Error>> {

}
