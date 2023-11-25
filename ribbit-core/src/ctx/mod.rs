// region:    --- Modules

mod error;

pub use self::error::{Error, Result};

// endregion: --- Modules

#[derive(Clone, Debug)]
pub struct Ctx {
    pub user_id: i64,
    pub user_name: String,
    //TODO add more fields, system_message
}

// Constructors.
impl Ctx {
    pub fn root_ctx() -> Self {
        Ctx {
            user_id: 0,
            user_name: "root".to_string(),
        }
    }

    pub fn new(user_id: i64, user_name: String) -> Result<Self> {
        if user_id == 0 {
            Err(Error::CtxCannotNewRootCtx)
        } else {
            Ok(Self { user_id, user_name })
        }
    }
}
