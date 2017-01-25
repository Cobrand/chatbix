pub type StdResult<T,E> = ::std::result::Result<T,E>;

pub fn iron_result<T>(result:Result<T>) -> ::iron::IronResult<T> {
    use iron::{status,IronError};
    result.map_err(|error: Error| {
        match error {
            Error(ErrorKind::ChronoParseError(parse_error),_) =>
                IronError::new(parse_error,(format!("{}",parse_error),status::UnprocessableEntity)),
            e => IronError::new(e, status::InternalServerError),
        }
    })
}

error_chain! {
    foreign_links {
        ChronoParseError(::chrono::ParseError);
    }
}
