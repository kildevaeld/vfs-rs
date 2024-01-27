#![no_std]
//! An ephemeral in-memory file system, intended mainly for unit tests
use vfs::{VFile, VPath, VFS};

extern crate alloc;

pub type Filename = String;

#[derive(Debug, Clone)]
pub struct DataHandle(pub(crate) Arc<RwLock<Vec<u8>>>);

impl DataHandle {
    fn new() -> DataHandle {
        DataHandle(Arc::new(RwLock::new(Vec::new())))
    }

    pub fn with_data(data: Vec<u8>) -> DataHandle {
        DataHandle(Arc::new(RwLock::new(data)))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum NodeKind {
    Directory,
    File,
}

#[derive(Debug)]
struct FsNode {
    kind: NodeKind,
    pub children: HashMap<String, FsNode>,
    pub data: DataHandle,
}

impl FsNode {
    pub fn new_directory() -> Self {
        FsNode {
            kind: NodeKind::Directory,
            children: HashMap::new(),
            data: DataHandle::new(),
        }
    }

    pub fn new_file() -> Self {
        FsNode {
            kind: NodeKind::File,
            children: HashMap::new(),
            data: DataHandle::new(),
        }
    }

    fn metadata(&mut self) -> Result<MemoryMetadata> {
        Ok(MemoryMetadata {
            kind: self.kind.clone(),
            len: self.data.0.read().unwrap().len() as u64,
        })
    }
}

#[derive(Debug)]
pub struct MemoryFSImpl {
    root: FsNode,
}

pub type MemoryFSHandle = Arc<RwLock<MemoryFSImpl>>;

/// An ephemeral in-memory file system, intended mainly for unit tests
#[derive(Debug, Clone)]
pub struct MemoryFS {
    handle: MemoryFSHandle,
}

impl MemoryFS {
    pub fn new() -> MemoryFS {
        MemoryFS {
            handle: Arc::new(RwLock::new(MemoryFSImpl {
                root: FsNode::new_directory(),
            })),
        }
    }
}

#[derive(Debug)]
pub struct MemoryFile {
    pub(crate) data: DataHandle,
    pub(crate) pos: u64,
}

impl Read for MemoryFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let data = self.data.0.write().unwrap();
        let n = (&data.deref()[self.pos as usize..]).read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl AsyncRead for MemoryFile {
    #[cfg(feature = "read-initializer")]
    unsafe fn initializer(&self) -> Initializer {
        io::Read::initializer(self)
    }

    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        Poll::Ready(io::Read::read(&mut *self, buf))
    }

    fn poll_read_vectored(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<Result<usize>> {
        Poll::Ready(io::Read::read_vectored(&mut *self, bufs))
    }
}

impl Write for MemoryFile {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut guard = self.data.0.write().unwrap();
        let ref mut vec: &mut Vec<u8> = guard.deref_mut();
        // From cursor.rs
        let pos = self.pos;
        let len = vec.len();
        let amt = pos.saturating_sub(len as u64);
        vec.resize(len + amt as usize, 0);
        {
            let pos = pos as usize;
            let space = vec.len() - pos;
            let (left, right) = buf.split_at(cmp::min(space, buf.len()));
            vec[pos..pos + left.len()].clone_from_slice(left);
            vec.extend_from_slice(right);
        }

        // Bump us forward
        self.pos = pos + buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        // Nothing to do
        Ok(())
    }
}

impl AsyncWrite for MemoryFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        Poll::Ready(Write::write(&mut *self, buf))
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize>> {
        Poll::Ready(Write::write_vectored(&mut *self, bufs))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Write::flush(&mut *self))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl VFile for MemoryFile {}

impl Seek for MemoryFile {
    fn seek(&mut self, style: SeekFrom) -> Result<u64> {
        let pos = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => {
                let data = self.data.0.read().unwrap();
                data.len() as i64 + n
            }
            SeekFrom::Current(n) => self.pos as i64 + n,
        };
        if pos < 0 {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid seek to a negative position",
            ))
        } else {
            self.pos = pos as u64;
            Ok(self.pos)
        }
    }
}

impl AsyncSeek for MemoryFile {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64>> {
        Poll::Ready(Seek::seek(&mut *self, pos))
    }
}

pub struct MemoryMetadata {
    kind: NodeKind,
    len: u64,
}

impl VMetadata for MemoryMetadata {
    fn is_dir(&self) -> bool {
        self.kind == NodeKind::Directory
    }
    fn is_file(&self) -> bool {
        self.kind == NodeKind::File
    }
    fn len(&self) -> u64 {
        self.len
    }
}

impl VFS for MemoryFS {
    type Path = MemoryPath;

    fn path(&self, path: &str) -> MemoryPath {
        MemoryPath::new(&self.handle, path.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPath {
    pub path: Filename,
    fs: MemoryFSHandle,
}

impl MemoryPath {
    pub fn new(fs: &MemoryFSHandle, path: Filename) -> Self {
        return MemoryPath {
            path: path,
            fs: fs.clone(),
        };
    }

    fn with_node<R, F: FnOnce(&mut FsNode) -> R>(&self, f: F) -> Result<R> {
        let root = &mut self.fs.write().unwrap().root;
        let mut components: Vec<&str> = self.path.split("/").collect();
        components.reverse();
        components.pop();
        return traverse_with(root, &mut components, f);
    }

    pub fn decompose_path(&self) -> (Option<String>, String) {
        let mut split = self.path.rsplitn(2, "/");
        if let Some(mut filename) = split.next() {
            if let Some(mut parent) = split.next() {
                if parent.is_empty() {
                    parent = "/";
                }
                if filename.is_empty() {
                    filename = parent;
                    return (None, filename.to_owned());
                }
                return (Some(parent.to_owned()), filename.to_owned());
            }
        }
        return (None, self.path.clone());
    }

    fn parent_internal(&self) -> Option<MemoryPath> {
        self.decompose_path()
            .0
            .map(|parent| MemoryPath::new(&self.fs.clone(), parent))
    }
}

fn traverse_mkdir(node: &mut FsNode, components: &mut Vec<&str>) -> Result<()> {
    if let Some(component) = components.pop() {
        let directory = &mut node
            .children
            .entry(component.to_owned())
            .or_insert_with(FsNode::new_directory);
        if directory.kind != NodeKind::Directory {
            return Err(Error::new(
                ErrorKind::Other,
                format!("File is not a directory: {}", component),
            ));
        }
        traverse_mkdir(directory, components)
    } else {
        Ok(())
    }
}

fn traverse_with<R, F: FnOnce(&mut FsNode) -> R>(
    node: &mut FsNode,
    components: &mut Vec<&str>,
    f: F,
) -> Result<R> {
    if let Some(component) = components.pop() {
        if component.is_empty() {
            return traverse_with(node, components, f);
        }
        let entry = node.children.get_mut(component);
        if let Some(directory) = entry {
            return traverse_with(directory, components, f);
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                format!("File not found {:?}", component),
            ));
        }
    } else {
        Ok(f(node))
    }
}

impl MemoryPath {
    pub(crate) fn open_with_options(&self, open_options: &OpenOptions) -> Result<MemoryFile> {
        let parent_path = match self.parent_internal() {
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("File is not a file: {:?}", self.file_name()),
                ));
            }
            Some(parent) => parent,
        };
        let data_handle = parent_path.with_node(|node| {
            let file_name = self.file_name().unwrap();
            let file_node_entry = node.children.entry(file_name);
            let file_node = match file_node_entry {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    if !open_options.create {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("File does not exist: {}", self.path),
                        ));
                    }
                    entry.insert(FsNode::new_file())
                }
            };
            if file_node.kind != NodeKind::File {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("File is not a file: {:?}", self.file_name()),
                ));
            }
            return Ok(file_node.data.clone());
        })??;
        if open_options.truncate {
            let mut data = data_handle.0.write().unwrap();
            data.clear();
        }
        let mut pos = 0u64;
        if open_options.append {
            pos = data_handle.0.read().unwrap().len() as u64;
        }

        Ok(MemoryFile {
            data: data_handle,
            pos: pos,
        })
    }

    pub(crate) fn create_dir_inner(&self) -> Result<()> {
        let root = &mut self.fs.write().unwrap().root;
        let mut components: Vec<&str> = self.path.split("/").collect();
        components.reverse();
        components.pop();
        traverse_mkdir(root, &mut components)
    }
}

#[async_trait]
impl VPath for MemoryPath {
    type Metadata = MemoryMetadata;
    type File = MemoryFile;
    type ReadDir = stream::Iter<<Vec<Result<MemoryPath>> as IntoIterator>::IntoIter>;

    fn parent(&self) -> Option<MemoryPath> {
        self.parent_internal()
    }

    fn file_name(&self) -> Option<String> {
        Some(self.decompose_path().1)
    }

    fn extension(&self) -> Option<String> {
        match self.file_name() {
            Some(name) => pathutils::extname(&name).map(|m| m.to_string()),
            None => None,
        }
    }

    fn resolve(&self, path: &str) -> MemoryPath {
        let mut new_path = self.path.clone();
        if !new_path.ends_with('/') {
            new_path.push_str("/");
        }
        new_path.push_str(&path);
        return MemoryPath::new(&self.fs, new_path);
    }

    async fn exists(&self) -> bool {
        self.with_node(|_node| ()).is_ok()
    }

    async fn metadata(&self) -> Result<MemoryMetadata> {
        match self.with_node(FsNode::metadata) {
            Ok(o) => o,
            Err(e) => Err(e),
        }
    }

    fn to_string(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::Owned(self.path.clone())
    }

    // fn to_path_buf(&self) -> Option<PathBuf> {
    //     None
    // }

    async fn open(&self, options: OpenOptions) -> Result<Self::File> {
        self.open_with_options(&options)
    }

    async fn read_dir(&self) -> Result<Self::ReadDir> {
        let children = self.with_node(|node| {
            let children: Vec<_> = node
                .children
                .keys()
                .map(|name| Ok(MemoryPath::new(&self.fs, self.path.clone() + "/" + name)))
                .collect();
            children
        });
        match children {
            Ok(children) => Ok(stream::iter(children.into_iter())),
            Err(e) => Err(e),
        }
    }

    async fn create_dir(&self) -> Result<()> {
        let root = &mut self.fs.write().unwrap().root;
        let mut components: Vec<&str> = self.path.split("/").collect();
        components.reverse();
        components.pop();
        traverse_mkdir(root, &mut components)
    }

    async fn rm(&self) -> Result<()> {
        let parent_path = match self.parent_internal() {
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("File is not a file: {:?}", self.file_name()),
                ))
            }
            Some(parent) => parent,
        };
        parent_path.with_node(|node| {
            let file_name = self.file_name().unwrap();
            node.children.remove(&file_name);
        })
    }

    async fn rm_all(&self) -> Result<()> {
        self.rm().await
    }
}

impl<'a> From<&'a MemoryPath> for String {
    fn from(path: &'a MemoryPath) -> String {
        path.path.clone()
    }
}

impl PartialEq for MemoryPath {
    fn eq(&self, other: &MemoryPath) -> bool {
        self.path == other.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    // use futures::StreamExt;
    // use std::io::{Read, Result, Seek, SeekFrom, Write};
    use VPath;
    use {VMetadata, VFS};

    #[test]
    fn mkdir() {
        block_on(async move {
            let fs = MemoryFS::new();
            let path = fs.path("/foo/bar/baz");
            assert!(!path.exists().await, "Path should not exist");
            path.create_dir().await.unwrap();
            assert!(path.exists().await, "Path should exist now");
            assert!(
                path.metadata().await.unwrap().is_dir(),
                "Path should be dir"
            );
            assert!(
                !path.metadata().await.unwrap().is_file(),
                "Path should be not be a file"
            );
            assert!(
                path.metadata().await.unwrap().len() == 0,
                "Path size should be 0"
            );
        });
    }

    #[test]
    fn mkdir_fails_for_file() {
        block_on(async {
            let fs = MemoryFS::new();
            let path = fs.path("/foo");
            path.open(OpenOptions::new().write(true).create(true).truncate(true))
                .await
                .unwrap();
            assert!(
                path.create_dir().await.is_err(),
                "Path should not be created"
            );
        });
    }

    /*#[tokio::test]
    async fn read_empty_file() {
        let fs = MemoryFS::new();
        let path = fs.path("/foobar.txt");
        path.open(OpenOptions::new().write(true).create(true).truncate(true))
            .await
            .unwrap();
        let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
        let mut string: String = "".to_owned();
        file.read_to_string(&mut string).unwrap();
        assert_eq!(string, "");
    }

    #[tokio::test]
    async fn rm() {
        let fs = MemoryFS::new();
        let path = fs.path("/foobar.txt");
        path.open(OpenOptions::new().write(true).create(true).truncate(true))
            .await
            .unwrap();
        path.rm().await.unwrap();
        assert!(!path.exists().await);
    }

    #[tokio::test]
    async fn rmdir() {
        let fs = MemoryFS::new();
        let path = fs.path("/foobar");
        path.mkdir().await.unwrap();
        path.rm().await.unwrap();
        assert!(!path.exists().await);
    }

    #[tokio::test]
    async fn rmrf() {
        let fs = MemoryFS::new();
        let dir = fs.path("/foo");
        dir.mkdir().await.unwrap();
        let path = fs.path("/foo/bar.txt");
        path.open(OpenOptions::new().write(true).create(true).truncate(true))
            .await
            .unwrap();
        dir.rm_all().await.unwrap();
        assert!(!path.exists().await);
        assert!(!dir.exists().await);
    }

    #[tokio::test]
    async fn access_directory_as_file() {
        let fs = MemoryFS::new();
        let path = fs.path("/foo");
        path.mkdir().await.unwrap();
        assert!(
            path.open(OpenOptions::new().write(true).create(true).truncate(true))
                .await
                .is_err(),
            "Directory should not be openable"
        );
        assert!(
            path.open(OpenOptions::new().write(true).create(true).append(true))
                .await
                .is_err(),
            "Directory should not be openable"
        );
        assert!(
            path.open(OpenOptions::new().read(true)).await.is_err(),
            "Directory should not be openable"
        );
    }

    #[tokio::test]
    async fn write_and_read_file() {
        let fs = MemoryFS::new();
        let path = fs.path("/foobar.txt");
        {
            let mut file = path
                .open(OpenOptions::new().write(true).create(true).truncate(true))
                .await
                .unwrap();
            write!(file, "Hello world").unwrap();
            write!(file, "!").unwrap();
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.read_to_string(&mut string).unwrap();
            assert_eq!(string, "Hello world!");
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            file.seek(SeekFrom::Start(1)).unwrap();
            write!(file, "a").unwrap();
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.read_to_string(&mut string).unwrap();
            assert_eq!(string, "Hallo world!");
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.seek(SeekFrom::End(-1)).unwrap();
            file.read_to_string(&mut string).unwrap();
            assert_eq!(string, "!");
        }
        {
            let _file = path
                .open(OpenOptions::new().write(true).create(true).truncate(true))
                .await
                .unwrap();
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.read_to_string(&mut string).unwrap();
            assert_eq!(string, "");
        }
    }

    #[tokio::test]
    async fn append() {
        let fs = MemoryFS::new();
        let path = fs.path("/foobar.txt");
        {
            let mut file = path
                .open(OpenOptions::new().write(true).create(true).append(true))
                .await
                .unwrap();
            write!(file, "Hello").unwrap();
            write!(file, " world").unwrap();
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.read_to_string(&mut string).unwrap();
            assert_eq!(string, "Hello world");
        }
        {
            let mut file = path
                .open(OpenOptions::new().write(true).create(true).append(true))
                .await
                .unwrap();
            write!(file, "!").unwrap();
        }
        {
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.read_to_string(&mut string).unwrap();
            assert_eq!(string, "Hello world!");
        }
    }
    #[tokio::test]
    async fn resolve() {
        let fs = MemoryFS::new();
        let path = fs.path("/");
        assert_eq!(path.to_string(), "/");
        let path2 = path.resolve(&"foo".to_string());
        assert_eq!(path2.to_string(), "/foo");
        let path3 = path2.resolve(&"bar".to_string());
        assert_eq!(path3.to_string(), "/foo/bar");

        assert_eq!(path.to_string(), "/");
        let path4 = path.resolve(&"foo/bar".to_string());
        assert_eq!(path4.to_string(), "/foo/bar");
    }

    #[tokio::test]
    async fn parent() {
        let fs = MemoryFS::new();
        let path = fs.path("/foo");
        let path2 = fs.path("/foo/bar");
        assert_eq!(path2.parent().unwrap().to_string(), path.to_string());
        assert_eq!(path.parent().unwrap().to_string(), "/");
        assert!(fs.path("/").parent().is_none());
    }

    #[tokio::test]
    async fn read_dir() {
        let fs = MemoryFS::new();
        let path = fs.path("/foo");
        let path2 = fs.path("/foo/bar");
        let path3 = fs.path("/foo/baz");
        path2.mkdir().await.unwrap();
        path3
            .open(OpenOptions::new().write(true).create(true).truncate(true))
            .await
            .unwrap();
        let mut entries: Vec<String> = path
            .read_dir()
            .await
            .unwrap()
            .map(Result::unwrap)
            .map(|path| path.to_string().into_owned())
            .collect()
            .await;
        entries.sort();
        assert_eq!(entries, vec!["/foo/bar".to_owned(), "/foo/baz".to_owned()]);
    }

    #[tokio::test]
    async fn file_name() {
        let fs = MemoryFS::new();
        let path = fs.path("/foo/bar.txt");
        assert_eq!(path.file_name(), Some("bar.txt".to_owned()));
        assert_eq!(path.extension(), Some(".txt".to_owned()));
        assert_eq!(path.parent().unwrap().extension(), None);
    }

    #[tokio::test]
    async fn path_buf() {
        let fs = MemoryFS::new();
        let path = fs.path("/foo/bar.txt");
        assert_eq!(None, path.to_path_buf());
    }
    */
}
