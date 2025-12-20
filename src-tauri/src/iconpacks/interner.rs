use std::{collections::HashMap, sync::RwLock};

// TODO: replace with a proper interner library
pub type StringId = u32;

pub struct StringInterner {
    map: RwLock<HashMap<String, StringId>>,
    vec: RwLock<Vec<String>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
            vec: RwLock::new(Vec::new()),
        }
    }

    pub fn clear(&self) {
        let mut map = self.map.write().unwrap();
        let mut vec = self.vec.write().unwrap();
        map.clear();
        vec.clear();
    }

    pub fn intern(&self, s: &str) -> StringId {
        if let Some(&id) = self.map.read().unwrap().get(s) {
            return id;
        }
        let id = self.vec.read().unwrap().len() as StringId;
        self.vec.write().unwrap().push(s.to_owned());
        self.map.write().unwrap().insert(s.to_owned(), id);

        id
    }

    pub fn resolve(&self, id: StringId) -> String {
        let vec = self.vec.read().unwrap();
        vec[id as usize].clone()
    }
}
