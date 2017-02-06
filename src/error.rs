use iron::{status,IronError,IronResult,Response};
use bodyparser::{BodyError, BodyErrorCause};

#[derive(Debug,Serialize)]
struct JsonError {
    status: &'static str,
    error: String,
}

impl JsonError {
    pub fn new(s: String) -> JsonError {
        JsonError {
            status: "error",
            error: s,
        }
    }
}

impl Into<IronResult<Response>> for Error {
    fn into(self) -> IronResult<Response> {
        let error = self;
        let (response_string, status) = match error {
            Error(ErrorKind::ChronoParseError(parse_error),_) =>
                (format!("{}",parse_error),status::UnprocessableEntity),
            Error(ErrorKind::InvalidCredentials, _) =>
                ("invalid username or password".to_owned(),status::UnprocessableEntity),
            Error(ErrorKind::InvalidAuthKey, _) =>
                ("invalid auth_key".to_owned(), status::Unauthorized),
            Error(ErrorKind::Forbidden, _) =>
                ("insufficient rights".to_owned(), status::Forbidden),
            Error(ErrorKind::NoJsonBodyDetected, _) => 
                ("no json body detected".to_owned(), status::BadRequest),
            Error(ErrorKind::NotLoggedIn, _) => 
                ("not logged in".to_owned(), status::Unauthorized),
            Error(ErrorKind::UsernameInUse, _) => 
                ("username already taken".to_owned(), status::Conflict),
            Error(ErrorKind::BodyparserError(body_error),_) =>
                match body_error.cause {
                    BodyErrorCause::Utf8Error(utf8_err) => (format!("body error: {}",utf8_err),status::UnprocessableEntity),
                    BodyErrorCause::IoError(io_error) => (format!("body error: {}",io_error),status::InternalServerError),
                    BodyErrorCause::JsonError(json_error) => (format!("body error: {}",json_error),status::UnprocessableEntity),
                },
            e => return Err(IronError::new(e, status::InternalServerError)),
        };
        let json_error = JsonError::new(response_string);
        Ok(Response::with((::serde_json::to_string(&json_error).unwrap(), status)))
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
        UsernameInUse
        Forbidden
        NotLoggedIn
        DatabaseBusy
        NoJsonBodyDetected
    }

    foreign_links {
        ChronoParseError(::chrono::ParseError);
        BodyparserError(BodyError);
    }
}
