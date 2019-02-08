pub mod boxed;
#[cfg(feature = "glob")]
pub mod glob;
pub mod memory;
pub mod physical;
mod traits;
mod utils;

pub use self::traits::*;
pub use self::utils::*;

pub mod prelude {
    #[cfg(feature = "glob")]
    pub use super::glob::*;
    pub use super::traits::*;
    pub use super::utils::*;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
