// region:   --- Modules

mod db_store;
mod error;
mod vec_store;

use std::collections::HashMap;

pub use self::error::{Error, Result};

pub use self::db_store::*;
pub use self::vec_store::*;
