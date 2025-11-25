use std::collections::HashMap;

pub struct Cache {
    data: HashMap<Vec<u8>, Vec<u8>>,
}

impl Cache {
    pub fn new() -> Self {
        let data = HashMap::new();
        Cache { data }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        self.data.insert(key, value)
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.data.get(key)
    }
}