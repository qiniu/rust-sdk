use super::super::storage::region::Region;
use chashmap::CHashMap;
use derive_builder::Builder;
use error_chain::error_chain;
use getset::{CopyGetters, Getters};
use rand::{rngs::ThreadRng, seq::SliceRandom};
use std::{
    boxed::Box,
    io::Error as IOError,
    mem,
    net::{SocketAddr, ToSocketAddrs},
    result,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Resolutions {
    socket_addrs: Box<[SocketAddr]>,
    deadline: Instant,
}

#[derive(Builder, Debug, Getters, CopyGetters)]
#[builder(
    name = "DomainsManagerBuilder",
    pattern = "owned",
    default,
    public,
    build_fn(name = "inner_build", private)
)]
pub struct DomainsManagerInner {
    #[builder(setter(skip))]
    frozen_urls: CHashMap<Box<str>, Instant>,
    #[builder(setter(skip))]
    resolutions: CHashMap<Box<str>, Resolutions>,

    #[get_copy = "pub"]
    frozen_urls_duration: Duration,

    #[get_copy = "pub"]
    resolutions_cache_lifetime: Duration,

    #[get_copy = "pub"]
    disable_url_resolution: bool,

    #[get = "pub"]
    pre_resolve_urls: Box<[&'static str]>,
}

impl Default for DomainsManagerInner {
    fn default() -> Self {
        DomainsManagerInner {
            frozen_urls: CHashMap::new(),
            resolutions: CHashMap::new(),
            frozen_urls_duration: Duration::from_secs(10 * 60),
            resolutions_cache_lifetime: Duration::from_secs(60 * 60),
            disable_url_resolution: false,
            pre_resolve_urls: {
                let mut urls = Vec::with_capacity(100);
                Region::all().iter().for_each(|region| {
                    urls.extend_from_slice(&region.up_urls(false));
                    urls.extend_from_slice(&region.up_urls(true));
                    urls.extend_from_slice(&region.io_urls(false));
                    urls.extend_from_slice(&region.io_urls(true));
                    urls.push(region.rs_url(false));
                    urls.push(region.rs_url(true));
                    urls.push(region.rsf_url(false));
                    urls.push(region.rsf_url(true));
                    urls.push(region.api_url(false));
                    urls.push(region.api_url(true));
                });
                urls.push(Region::uc_url(false));
                urls.push(Region::uc_url(true));
                urls.into()
            },
        }
    }
}

impl DomainsManagerBuilder {
    pub fn build(self) -> result::Result<DomainsManager, String> {
        self.inner_build().map(|inner| {
            let domains_manager = DomainsManager(Arc::new(inner));
            if !domains_manager.0.pre_resolve_urls().is_empty() {
                domains_manager.async_pre_resolve_urls();
            }
            domains_manager
        })
    }
}

#[derive(Debug, Clone)]
pub struct DomainsManager(Arc<DomainsManagerInner>);

impl DomainsManager {
    pub fn choose<'a>(&self, urls: &'a [&'a str]) -> Result<Vec<Choice<'a>>> {
        let mut rng = rand::thread_rng();
        assert!(!urls.is_empty());
        let mut choices = Vec::<Choice>::with_capacity(urls.len());
        for url in urls.into_iter() {
            if !self.is_frozen_url(url)? {
                if let Some(choice) = self.make_choice(url, &mut rng) {
                    choices.push(choice);
                }
            }
        }
        if choices.is_empty() {
            let now = Instant::now();
            choices.push(
                urls.into_iter()
                    .filter_map(|url| self.make_choice(url, &mut rng))
                    .min_by_key(|choice| {
                        self.0
                            .frozen_urls
                            .get(&Self::host_with_port(choice.url).unwrap())
                            .map(|time| time.duration_since(now))
                            .unwrap_or_else(|| Duration::from_secs(0))
                    })
                    .unwrap(),
            );
        }

        Ok(choices)
    }

    pub fn freeze_url(&self, url: &str) -> Result<()> {
        self.0
            .frozen_urls
            .insert(Self::host_with_port(url)?, Instant::now() + self.0.frozen_urls_duration);
        Ok(())
    }

    pub fn unfreeze_urls(&self) {
        self.0.frozen_urls.clear();
    }

    pub fn is_frozen_url(&self, url: &str) -> Result<bool> {
        let url = Self::host_with_port(url)?;
        match self.0.frozen_urls.get(&url) {
            Some(unfreeze_time) => {
                if *unfreeze_time < Instant::now() {
                    mem::drop(unfreeze_time);
                    self.0.frozen_urls.remove(&url);
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            None => Ok(false),
        }
    }

    fn make_choice<'a>(&self, url: &'a str, rng: &mut ThreadRng) -> Option<Choice<'a>> {
        if self.0.disable_url_resolution {
            return Some(Choice {
                url: url,
                socket_addrs: Vec::new().into(),
            });
        }
        self.resolve(url)
            .ok()
            .map(|mut results| {
                // TODO: Think about IP address speed testing
                results.shuffle(rng);
                results
            })
            .map(|results| Choice {
                url: url,
                socket_addrs: results,
            })
    }

    fn resolve(&self, url: &str) -> Result<Box<[SocketAddr]>> {
        let url = Self::host_with_port(url)?;
        match self.0.resolutions.get(&url) {
            Some(resolution) => {
                if resolution.deadline < Instant::now() {
                    mem::drop(resolution);
                    self.resolve_and_update_cache(&url)
                } else {
                    Ok(resolution.socket_addrs.clone())
                }
            }
            None => self.resolve_and_update_cache(&url),
        }
    }

    fn resolve_and_update_cache(&self, url: &str) -> Result<Box<[SocketAddr]>> {
        let mut result: Option<Result<Box<[SocketAddr]>>> = None;
        self.0.resolutions.alter(url.into(), |resolutions| match resolutions {
            Some(resolutions) => {
                if resolutions.deadline < Instant::now() {
                    match self.make_resolutions(url) {
                        Ok(resolutions) => {
                            result = Some(Ok(resolutions.socket_addrs.clone()));
                            Some(resolutions)
                        }
                        Err(err) => {
                            result = Some(Err(err));
                            None
                        }
                    }
                } else {
                    result = Some(Ok(resolutions.socket_addrs.clone()));
                    Some(resolutions)
                }
            }
            None => match self.make_resolutions(url) {
                Ok(resolutions) => {
                    result = Some(Ok(resolutions.socket_addrs.clone()));
                    Some(resolutions)
                }
                Err(err) => {
                    result = Some(Err(err));
                    None
                }
            },
        });
        result.unwrap()
    }

    fn make_resolutions(&self, url: &str) -> Result<Resolutions> {
        Ok(Resolutions {
            socket_addrs: url.to_socket_addrs()?.collect::<Box<[_]>>().clone(),
            deadline: Instant::now() + self.0.resolutions_cache_lifetime,
        })
    }

    fn host_with_port(url: &str) -> Result<Box<str>> {
        url::Url::parse(&url)
            .ok()
            .and_then(|url| {
                url.host_str()
                    .map(|host| host.to_owned() + ":" + &url.port_or_known_default().unwrap().to_string())
                    .map(|host_with_port| host_with_port.into())
            })
            .ok_or_else(|| ErrorKind::InvalidURL(url.into()).into())
    }

    fn async_pre_resolve_urls(&self) {
        let domains_manager = self.clone();
        thread::spawn(move || {
            println!(domains_manager.0.pre_resolve_urls());
            for url in domains_manager.0.pre_resolve_urls().into_iter() {
                domains_manager.resolve(*url).ok();
            }
        });
    }
}

impl Default for DomainsManager {
    fn default() -> Self {
        DomainsManagerBuilder::default().build().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct Choice<'a> {
    pub url: &'a str,
    pub socket_addrs: Box<[SocketAddr]>,
}

error_chain! {
    errors {
        InvalidURL(d: Box<str>) {
            description("Invalid url")
            display("Invalid url: {}", d)
        }
    }

    foreign_links {
        ResolveError(IOError);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, error::Error, result::Result, thread};

    #[test]
    fn test_domains_manager_in_multiple_threads() -> Result<(), Box<dyn Error>> {
        let domains_manager = DomainsManagerBuilder::default()
            .frozen_urls_duration(Duration::from_secs(5))
            .build()
            .unwrap();
        assert!(!domains_manager.is_frozen_url("http://up.qiniup.com")?);

        let mut threads: Vec<thread::JoinHandle<()>> = Vec::with_capacity(10);
        {
            {
                let domains_manager = domains_manager.clone();
                threads.push(thread::Builder::new().name("thread0".into()).spawn(move || {
                    assert!(!domains_manager.is_frozen_url("http://up.qiniup.com").unwrap());

                    domains_manager.freeze_url("http://up.qiniup.com").unwrap();
                    assert!(domains_manager.is_frozen_url("http://up.qiniup.com").unwrap());

                    thread::sleep(Duration::from_secs(1));

                    domains_manager.freeze_url("http://upload.qiniup.com").unwrap();
                    assert!(domains_manager.is_frozen_url("http://upload.qiniup.com").unwrap());
                })?);
            }
            for thread_id in 1..=9 {
                let domains_manager = domains_manager.clone();
                threads.push(
                    thread::Builder::new()
                        .name(format!("thread{}", thread_id))
                        .spawn(move || {
                            assert!(!domains_manager.is_frozen_url("http://upload.qiniup.com").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(domains_manager.is_frozen_url("http://up.qiniup.com").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(domains_manager.is_frozen_url("http://up.qiniup.com").unwrap());
                            assert!(domains_manager.is_frozen_url("http://upload.qiniup.com/abc").unwrap());
                            assert!(!domains_manager.is_frozen_url("https://up.qiniup.com").unwrap());
                            assert!(!domains_manager.is_frozen_url("https://upload.qiniup.com/abc").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(domains_manager.is_frozen_url("http://up.qiniup.com/").unwrap());
                            assert!(domains_manager.is_frozen_url("http://upload.qiniup.com").unwrap());
                            thread::sleep(Duration::from_millis(2500));
                            assert!(!domains_manager
                                .is_frozen_url("http://up.qiniup.com/def/fgh.xzy")
                                .unwrap());
                            assert!(!domains_manager.is_frozen_url("http://up.qiniup.com/").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(!domains_manager.is_frozen_url("http://up.qiniup.com/").unwrap());
                            thread::sleep(Duration::from_secs(1));
                            assert!(!domains_manager
                                .is_frozen_url("http://upload.qiniup.com/def/fgh.xzy")
                                .unwrap());
                        })?,
                );
            }
        }
        threads.into_iter().for_each(|thread| thread.join().unwrap());
        Ok(())
    }

    #[test]
    fn test_domains_manager_choose() -> Result<(), Box<dyn Error>> {
        let domains_manager = DomainsManager::default();
        domains_manager.freeze_url("http://up-z0.qiniup.com").unwrap();
        domains_manager.freeze_url("http://up-z1.qiniup.com").unwrap();

        let choices = domains_manager.choose(&["http://up-z0.qiniup.com", "http://up-z1.qiniup.com"])?;
        assert_eq!(choices.len(), 1);
        assert_eq!(choices.first().unwrap().url, "http://up-z0.qiniup.com");
        assert!(choices.first().unwrap().socket_addrs.len() > 5);

        let choices = domains_manager.choose(&[
            "http://up-z1.qiniup.com",
            "http://up-z2.qiniup.com",
            "http://unexisted-z3.qiniup.com",
            "http://unexisted-z4.qiniup.com",
        ])?;
        assert_eq!(choices.len(), 1);
        assert_eq!(choices.first().unwrap().url, "http://up-z2.qiniup.com");
        assert!(choices.first().unwrap().socket_addrs.len() > 5);
        Ok(())
    }
}
