use iron::{status,IronError,IronResult,Response};

pub type StdResult<T,E> = ::std::result::Result<T,E>;

impl Into<IronResult<Response>> for Error {
    fn into(self) -> IronResult<Response> {
        let error = self;
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
    }
}

macro_rules! chatbix_try {
    ($expr:expr) => (
        match $expr {
            Ok(val) => val,
            Err(error) => return error.into()
        }
    )
}

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
