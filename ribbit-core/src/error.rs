pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // config
    ConfigMissingEnv(&'static str),
    ConfigParseInt { var_name: String },
    ConfigParseConfigFile(String),
    ConfigReadConfigFile(String),
    DotEnvNotFound,
}

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}
impl std::error::Error for Error {}
// endregion: --- Error Boilerplate
