use alloc::{string::String, vec::Vec};
use udled::Tokenizer;

pub enum Component {
    Literal(String),
    // Basename(String),
    // Exension(String),
    Parent,
}

pub struct Pattern {
    components: Vec<Component>,
}

impl Pattern {
    pub fn matches<V: vfs::VPath>(&self, path: V) -> bool {
        let mut current_path = path;
        for component in self.components.iter() {
            match component {
                Component::Literal(lit) => {}
                Component::Parent => {
                    let Some(parent) = current_path.parent() else {
                        return false;
                    };
                    current_path = parent;
                }
            }
        }

        todo!()
    }
}
