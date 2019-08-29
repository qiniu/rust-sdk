use error_chain::error_chain;
use std::{
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct DomainsManager {
    map: Arc<chashmap::CHashMap<String, Instant>>,
}

impl DomainsManager {
    pub fn new() -> Self {
        DomainsManager {
            map: Arc::new(chashmap::CHashMap::new()),
        }
    }

    pub(crate) fn is_frozen<D: Into<String>>(&self, domain: D) -> Result<bool> {
        let domain = Self::normalize_domain(domain)?;
        match self.map.get(&domain) {
            Some(unfreeze_time) => {
                if *unfreeze_time < Instant::now() {
                    mem::drop(unfreeze_time);
                    self.map.remove(&domain);
                    return Ok(false);
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn freeze<D: Into<String>>(&self, domain: D, frozen_seconds: Duration) -> Result<()> {
        self.map
            .insert(Self::normalize_domain(domain)?, Instant::now() + frozen_seconds);
        Ok(())
    }

    pub fn unfreeze_all(&self) {
        self.map.clear();
    }

    fn normalize_domain<D: Into<String>>(domain: D) -> Result<String> {
        let domain = domain.into();
        match url::Url::parse(&domain) {
            Ok(url) => url
                .host_str()
                .map(|h| h.to_string())
                .ok_or_else(|| ErrorKind::InvalidDomain(domain).into()),
            Err(err) => match err {
                url::ParseError::RelativeUrlWithoutBase => {
                    let domain_with_scheme = "http://".to_owned() + &domain;
                    match url::Url::parse(&domain_with_scheme) {
                        Ok(url) => url
                            .host_str()
                            .map(|h| h.to_string())
                            .ok_or_else(|| ErrorKind::InvalidDomain(domain).into()),
                        Err(_) => Err(ErrorKind::InvalidDomain(domain).into()),
                    }
                }
                _ => Err(ErrorKind::InvalidDomain(domain).into()),
            },
        }
    }
}

error_chain! {
    errors {
        InvalidDomain(d: String) {
            description("Invalid domain")
            display("Invalid domain: {}", d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_domains_manager_in_multiple_threads() {
        let domains_manager = DomainsManager::new();
        assert!(!domains_manager.is_frozen("up.qiniup.com").unwrap());

        let mut threads: Vec<thread::JoinHandle<()>> = Vec::with_capacity(10);
        {
            let dm = domains_manager.clone();
            threads.push(
                thread::Builder::new()
                    .name("thread0".into())
                    .spawn(move || {
                        assert!(!dm.is_frozen("up.qiniup.com").unwrap());

                        dm.freeze("up.qiniup.com", Duration::from_secs(5)).unwrap();
                        assert!(dm.is_frozen("up.qiniup.com").unwrap());

                        thread::sleep(Duration::from_secs(1));

                        dm.freeze("upload.qiniup.com", Duration::from_secs(5)).unwrap();
                        assert!(dm.is_frozen("upload.qiniup.com").unwrap());
                    })
                    .unwrap(),
            );
            for thread_id in 1..=9 {
                let dm = domains_manager.clone();
                threads.push(
                    thread::Builder::new()
                        .name(format!("thread{}", thread_id))
                        .spawn(move || {
                            assert!(!dm.is_frozen("upload.qiniup.com").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(dm.is_frozen("http://up.qiniup.com").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(dm.is_frozen("https://up.qiniup.com").unwrap());
                            assert!(dm.is_frozen("https://upload.qiniup.com/abc").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(dm.is_frozen("up.qiniup.com/").unwrap());
                            assert!(dm.is_frozen("http://upload.qiniup.com").unwrap());
                            thread::sleep(Duration::from_millis(2500));
                            assert!(!dm.is_frozen("up.qiniup.com/def/fgh.xzy").unwrap());
                            assert!(!dm.is_frozen("https://up.qiniup.com/").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(!dm.is_frozen("https://up.qiniup.com/").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(!dm.is_frozen("https://upload.qiniup.com/def/fgh.xzy").unwrap());
                        })
                        .unwrap(),
                );
            }
        }
        for thread in threads {
            thread.join().unwrap();
        }
    }
}
