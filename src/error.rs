use iron::{status,IronError,Response};

pub type StdResult<T,E> = ::std::result::Result<T,E>;

macro_rules! chatbix_try {
    ($expr:expr) => (
        match $expr {
            Ok(val) => val,
            Err(error) => return match error {
                Error(ErrorKind::ChronoParseError(parse_error),_) =>
                    Ok(Response::with((format!("{}",parse_error),status::UnprocessableEntity))),
                Error(ErrorKind::InvalidCredentials, _) =>
                    Ok(Response::with(("invalid username or password",status::UnprocessableEntity))),
                Error(ErrorKind::InvalidAuthKey, _) =>
                    Ok(Response::with(("invalid auth_key",status::Unauthorized))),
                Error(ErrorKind::MissingParameter(missing_param_name), _) =>
                    Ok(Response::with((format!("missing parameter `{}`",missing_param_name),status::UnprocessableEntity))),
                e => Err(IronError::new(e, status::InternalServerError)),
            }
        }
    )
}

/*
pub fn iron_result(result:Result<Response>) -> ::iron::IronResult<Response> {
    result.or_else(|error: Error| {
        // IronError are meant to be logged, they are typically 5XX errors
        // Errors converted as Responses can be safely ignored and must not be
        // logged, they are typically 4XX errors
        match error {
            Error(ErrorKind::ChronoParseError(parse_error),_) =>
                Ok(Response::with((format!("{}",parse_error),status::UnprocessableEntity))),
            Error(ErrorKind::InvalidCredentials, _) =>
                Ok(Response::with(("invalid username or password",status::UnprocessableEntity))),
            Error(ErrorKind::InvalidAuthKey, _) =>
                Ok(Response::with(("invalid auth_key",status::Unauthorized))),
            Error(ErrorKind::MissingParameter(missing_param_name), _) =>
                Ok(Response::with((format!("missing parameter `{}`",missing_param_name),status::UnprocessableEntity))),
            e => Err(IronError::new(e, status::InternalServerError)),
        }
    })
}
*/

error_chain! {
    errors {
        InvalidCredentials
        InvalidAuthKey
        MissingParameter(s: String)
        DatabaseBusy
    }

    foreign_links {
        ChronoParseError(::chrono::ParseError);
    }
}
