use crate::RespFrame;
use dashmap::{DashMap, DashSet};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Backend(Arc<BackendInner>);

#[derive(Debug)]
pub struct BackendInner {
    pub(crate) map: DashMap<String, RespFrame>,
    pub(crate) hmap: DashMap<String, DashMap<String, RespFrame>>,
    pub(crate) set: DashMap<String, DashSet<String>>,
}

impl Deref for Backend {
    type Target = BackendInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self(Arc::new(BackendInner::default()))
    }
}

impl Default for BackendInner {
    fn default() -> Self {
        Self {
            map: DashMap::new(),
            hmap: DashMap::new(),
            set: DashMap::new(),
        }
    }
}

impl Backend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sadd(&self, key: String, members: Vec<String>) -> i64 {
        let mut count = 0;
        let set = self.set.entry(key).or_default();

        members.into_iter().for_each(|member| {
            if set.insert(member) {
                count += 1
            }
        });
        count
    }

    pub fn sismember(&self, key: &str, member: &str) -> i64 {
        if self.set.get(key).map_or(false, |set| set.contains(member)) {
            1
        } else {
            0
        }
    }

    pub fn get(&self, key: &str) -> Option<RespFrame> {
        self.map.get(key).map(|v| v.value().clone())
    }

    pub fn set(&self, key: String, value: RespFrame) {
        self.map.insert(key, value);
    }

    pub fn hget(&self, key: &str, field: &str) -> Option<RespFrame> {
        self.hmap
            .get(key)
            .and_then(|v| v.get(field).map(|v| v.value().clone()))
    }

    pub fn hmget(&self, key: &str, fields: &Vec<String>) -> Vec<Option<RespFrame>> {
        let mut resp: Vec<Option<RespFrame>> = vec![];

        for field in fields {
            if let Some(hashmap) = self.hmap.get(key) {
                if let Some(value) = hashmap.get(field) {
                    resp.push(Some(value.clone()));
                } else {
                    resp.push(None);
                }
            } else {
                resp.push(None);
            }
        }

        resp
    }

    pub fn hset(&self, key: String, field: String, value: RespFrame) {
        let hmap = self.hmap.entry(key).or_default();
        hmap.insert(field, value);
    }

    pub fn hgetall(&self, key: &str) -> Option<DashMap<String, RespFrame>> {
        self.hmap.get(key).map(|v| v.clone())
    }
}
