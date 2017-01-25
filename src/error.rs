pub type StdResult<T,E> = ::std::result::Result<T,E>;

error_chain! {
    foreign_links {
        ChronoParseError(::chrono::ParseError);
    }
}
