pub type StdResult<T,E> = ::std::result::Result<T,E>;

pub fn iron_result<T>(result:Result<T>) -> ::iron::IronResult<T> {
    use iron::{status,IronError};
    result.map_err(|error: Error| {
        match error {
            //Error::ChronoParseError(parse_error) =>
            //    IronError::new(parse_error,("unparsable datetime",status::UnprocessableEntity)),
            e => IronError::new(e, status::InternalServerError),
        }
    })
}

error_chain! {
    foreign_links {
        ChronoParseError(::chrono::ParseError);
    }
}
