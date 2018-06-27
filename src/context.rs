use config::Config;
use lru_cache::LruCache;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct Context {
    config: Arc<Config>,
    cache: Arc<Mutex<LruCache<String, bool>>>,
}

impl Context {
    pub fn new(config: Config) -> Self {
        Context {
            config: Arc::new(config),
            cache: Arc::new(Mutex::new(LruCache::new(99999))),
        }
    }
    pub fn local(&self) -> &SocketAddr {
        &self.config.local
    }
    pub fn proxy(&self) -> &SocketAddr {
        &self.config.proxy
    }
    pub fn connect_timeout(&self, addr: &SocketAddr) -> Option<u64> {
        if !self.config.autoproxy || self.contains(addr) {
            None
        } else {
            self.config.timeout
        }
    }
    pub fn contains(&self, addr: &SocketAddr) -> bool {
        let mut cache = self.cache.lock().unwrap();
        let addr_str = format!("{:?}", addr);
        cache.contains_key(&addr_str)
    }
    pub fn put_addr(&self, addr: &SocketAddr) {
        let mut cache = self.cache.lock().unwrap();
        let addr_str = format!("{:?}", addr);
        cache.insert(addr_str, true);
    }
}
