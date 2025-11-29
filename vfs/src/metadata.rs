#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Dir,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Metadata {
    pub size: u64,
    pub kind: FileType,
}

impl Metadata {
    pub fn is_file(&self) -> bool {
        matches!(self.kind, FileType::File)
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.kind, FileType::Dir)
    }
}
