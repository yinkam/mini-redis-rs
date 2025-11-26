use std::collections::HashMap;
use std::time::Instant;

pub struct Cache {
    data: HashMap<Vec<u8>, (Vec<u8>, Option<Instant>)>,
}

impl Cache {
    pub fn new() -> Self {
        let data = HashMap::new();
        Cache { data }
    }

    pub fn insert(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
        expiry_time: Option<Instant>,
    ) -> Option<(Vec<u8>, Option<Instant>)> {
        self.data.insert(key, (value, expiry_time))
    }

    pub fn get(&mut self, key: &Vec<u8>) -> Option<Vec<u8>> {
        if let Some((value, expiry)) = self.data.get(key) {
            if let Some(exp_time) = expiry {
                if Instant::now() > *exp_time {
                    self.data.remove(key);
                    return None;
                }
            }
            return Some(value.clone());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_insert_expirable() {
        let mut cache = Cache::new();
        let key = b"key".to_vec();
        let value = b"value".to_vec();
        let duration = Duration::from_secs(1);
        let expiry_time = Instant::now() + duration;
        cache.insert(key.clone(), value.clone(), Some(expiry_time));
        thread::sleep(duration / 2);
        assert_eq!(cache.get(&key), Some(value));

        thread::sleep(duration / 2);
        assert_eq!(cache.get(&key), None);
    }
}
