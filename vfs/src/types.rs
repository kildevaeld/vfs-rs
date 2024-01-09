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

    pub fn id_dir(&self) -> bool {
        matches!(self.kind, FileType::Dir)
    }
}

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    /// Sets the offset to the provided number of bytes.
    Start(u64),

    /// Sets the offset to the size of this object plus the specified number of
    /// bytes.
    ///
    /// It is possible to seek beyond the end of an object, but it's an error to
    /// seek before byte 0.
    End(i64),

    /// Sets the offset to the current position plus the specified number of
    /// bytes.
    ///
    /// It is possible to seek beyond the end of an object, but it's an error to
    /// seek before byte 0.
    Current(i64),
}

#[cfg(feature = "std")]
impl From<std::io::SeekFrom> for SeekFrom {
    fn from(value: std::io::SeekFrom) -> Self {
        match value {
            std::io::SeekFrom::Start(i) => SeekFrom::Start(i),
            std::io::SeekFrom::End(i) => SeekFrom::End(i),
            std::io::SeekFrom::Current(i) => SeekFrom::Current(i),
        }
    }
}

#[cfg(feature = "std")]
impl From<SeekFrom> for std::io::SeekFrom {
    fn from(value: SeekFrom) -> Self {
        match value {
            SeekFrom::Start(i) => std::io::SeekFrom::Start(i),
            SeekFrom::End(i) => std::io::SeekFrom::End(i),
            SeekFrom::Current(i) => std::io::SeekFrom::Current(i),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct OpenOptions {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub append: bool,
    pub truncate: bool,
}

impl OpenOptions {
    /// Create a new instance
    pub fn new() -> OpenOptions {
        Default::default()
    }

    /// Open for reading
    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Open for writing
    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Create the file if it does not exist yet
    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Append at the end of the file
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Truncate the file to 0 bytes after opening
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }
}

#[cfg(feature = "std")]
impl From<OpenOptions> for std::fs::OpenOptions {
    fn from(value: OpenOptions) -> Self {
        let mut ops = std::fs::OpenOptions::new();

        ops.append(value.append)
            .read(value.read)
            .write(value.write)
            .truncate(value.truncate)
            .create(value.create);

        ops
    }
}
