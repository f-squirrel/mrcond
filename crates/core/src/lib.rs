pub mod error;
pub mod publish;

pub use error::Error;
pub use publish::Publish;

pub type Result<T> = std::result::Result<T, Error>;
