mod boxed;
mod path_ext;
mod types;

#[cfg(feature = "util")]
pub mod util;

pub use self::{
    boxed::{vfs_box, vpath_box, VFSBox, VFileBox, VMetadataBox, VPathBox},
    path_ext::VPathExt,
    types::*,
};
