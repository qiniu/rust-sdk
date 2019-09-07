use error_chain::error_chain;
use std::{
    boxed::Box,
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct DomainsManager {
    map: Arc<chashmap::CHashMap<Box<str>, Instant>>,
}

impl DomainsManager {
    pub fn new() -> Self {
        DomainsManager {
            map: Arc::new(chashmap::CHashMap::new()),
        }
    }

    pub fn is_frozen<D: AsRef<str>>(&self, domain: D) -> Result<bool> {
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

    pub fn freeze<D: AsRef<str>>(&self, domain: D, frozen_seconds: Duration) -> Result<()> {
        self.map
            .insert(Self::normalize_domain(domain)?, Instant::now() + frozen_seconds);
        Ok(())
    }

    pub fn unfreeze_all(&self) {
        self.map.clear();
    }

    fn normalize_domain<D: AsRef<str>>(domain: D) -> Result<Box<str>> {
        let domain = domain.as_ref();
        match url::Url::parse(&domain) {
            Ok(url) => url
                .host_str()
                .map(|h| h.into())
                .ok_or_else(|| ErrorKind::InvalidDomain(domain.into()).into()),
            Err(err) => match err {
                url::ParseError::RelativeUrlWithoutBase => {
                    let domain_with_scheme = "http://".to_owned() + &domain;
                    match url::Url::parse(&domain_with_scheme) {
                        Ok(url) => url
                            .host_str()
                            .map(|h| h.into())
                            .ok_or_else(|| ErrorKind::InvalidDomain(domain.into()).into()),
                        Err(_) => Err(ErrorKind::InvalidDomain(domain.into()).into()),
                    }
                }
                _ => Err(ErrorKind::InvalidDomain(domain.into()).into()),
            },
        }
    }
}

error_chain! {
    errors {
        InvalidDomain(d: Box<str>) {
            description("Invalid domain")
            display("Invalid domain: {}", d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, error::Error, result::Result, thread};

    #[test]
    fn test_domains_manager_in_multiple_threads() -> Result<(), Box<dyn Error>> {
        let domains_manager = DomainsManager::new();
        assert!(!domains_manager.is_frozen("up.qiniup.com")?);

        let mut threads: Vec<thread::JoinHandle<()>> = Vec::with_capacity(10);
        {
            {
                let domains_manager = domains_manager.clone();
                threads.push(thread::Builder::new().name("thread0".into()).spawn(move || {
                    assert!(!domains_manager.is_frozen("up.qiniup.com").unwrap());

                    domains_manager.freeze("up.qiniup.com", Duration::from_secs(5)).unwrap();
                    assert!(domains_manager.is_frozen("up.qiniup.com").unwrap());

                    thread::sleep(Duration::from_secs(1));

                    domains_manager
                        .freeze("upload.qiniup.com", Duration::from_secs(5))
                        .unwrap();
                    assert!(domains_manager.is_frozen("upload.qiniup.com").unwrap());
                })?);
            }
            for thread_id in 1..=9 {
                let domains_manager = domains_manager.clone();
                threads.push(
                    thread::Builder::new()
                        .name(format!("thread{}", thread_id))
                        .spawn(move || {
                            assert!(!domains_manager.is_frozen("upload.qiniup.com").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(domains_manager.is_frozen("http://up.qiniup.com").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(domains_manager.is_frozen("https://up.qiniup.com").unwrap());
                            assert!(domains_manager.is_frozen("https://upload.qiniup.com/abc").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(domains_manager.is_frozen("up.qiniup.com/").unwrap());
                            assert!(domains_manager.is_frozen("http://upload.qiniup.com").unwrap());
                            thread::sleep(Duration::from_millis(2500));
                            assert!(!domains_manager.is_frozen("up.qiniup.com/def/fgh.xzy").unwrap());
                            assert!(!domains_manager.is_frozen("https://up.qiniup.com/").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(!domains_manager.is_frozen("https://up.qiniup.com/").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(!domains_manager
                                .is_frozen("https://upload.qiniup.com/def/fgh.xzy")
                                .unwrap());
                        })?,
                );
            }
        }
        threads.into_iter().for_each(|thread| thread.join().unwrap());
        Ok(())
    }
}
