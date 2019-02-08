use super::boxed::GPath;

impl<T: ?Sized> GPathExt for T where T: GPath {}

pub trait GPathExt: GPath {
    fn walk_dir(&self) -> WalkDirIter {
        WalkDirIter {
            todo: vec![self.box_clone()],
        }
    }
}

pub struct WalkDirIter {
    todo: Vec<Box<GPath>>,
}

impl Iterator for WalkDirIter {
    type Item = Box<GPath>;
    // TODO: handle loops
    fn next(&mut self) -> Option<Box<GPath>> {
        let res = self.todo.pop();
        if let Some(ref path) = res {
            if let Ok(metadata) = path.metadata() {
                if metadata.is_dir() {
                    if let Ok(entries) = path.read_dir() {
                        for entry in entries {
                            if let Ok(child) = entry {
                                self.todo.push(child);
                            }
                        }
                    }
                }
            }
        }
        res
    }
}
