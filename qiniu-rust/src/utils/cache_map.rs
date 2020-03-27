use chashmap::{CHashMap, IntoIter as CIntoIter, ReadGuard as CReadGuard, WriteGuard as CWriteGuard};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    cmp::PartialEq,
    hash::Hash,
    iter::Iterator,
    ops::{Deref, DerefMut},
    result::Result,
    time::SystemTime,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry<Data> {
    expired_at: SystemTime,
    data: Data,
}
#[derive(Default, Debug, Clone)]
pub struct CacheMap<K, V> {
    map: CHashMap<K, Entry<V>>,
    auto_clean: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistentEntry<K, V> {
    key: K,
    value: V,
    expired_at: SystemTime,
}
pub struct IntoIter<K, V>(CIntoIter<K, Entry<V>>);
#[derive(Debug)]
pub struct ReadGuard<'a, K: 'a, V: 'a>(CReadGuard<'a, K, Entry<V>>);
#[derive(Debug)]
pub struct WriteGuard<'a, K: 'a, V: 'a>(CWriteGuard<'a, K, Entry<V>>);

enum ForEachSelector {
    Effective,
    Expired,
    All,
}

impl<K, V> CacheMap<K, V> {
    #[inline]
    pub fn with_capacity(cap: usize, auto_clean: bool) -> Self {
        CacheMap {
            map: CHashMap::with_capacity(cap),
            auto_clean,
        }
    }

    #[inline]
    pub fn new(auto_clean: bool) -> Self {
        CacheMap {
            map: CHashMap::new(),
            auto_clean,
        }
    }

    #[inline]
    pub fn auto_clean(&self) -> bool {
        self.auto_clean
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    pub fn clear(&self) -> Self {
        CacheMap {
            map: self.map.clear(),
            auto_clean: self.auto_clean,
        }
    }

    #[inline]
    pub fn for_each_effective(&self, handler: impl Fn(&K, &V, &SystemTime)) {
        self.for_each_inner(ForEachSelector::Effective, handler)
    }

    #[inline]
    pub fn for_each_expired(&self, handler: impl Fn(&K, &V, &SystemTime)) {
        self.for_each_inner(ForEachSelector::Expired, handler)
    }

    #[inline]
    pub fn for_each(&self, handler: impl Fn(&K, &V, &SystemTime)) {
        self.for_each_inner(ForEachSelector::All, handler)
    }

    fn for_each_inner(&self, selector: ForEachSelector, handler: impl Fn(&K, &V, &SystemTime)) {
        let now = SystemTime::now();
        self.map.retain(|key, value| {
            let matched = match selector {
                ForEachSelector::Effective => value.expired_at > now,
                ForEachSelector::Expired => value.expired_at <= now,
                ForEachSelector::All => true,
            };
            if matched {
                handler(key, &value.data, &value.expired_at);
            }
            if self.auto_clean {
                value.expired_at > now
            } else {
                true
            }
        })
    }

    pub fn into_persistent(self) -> Vec<PersistentEntry<K, V>> {
        self.into_iter()
            .map(|(key, value, expired_at)| PersistentEntry { key, value, expired_at })
            .collect()
    }
}

impl<K: PartialEq + Hash, V> CacheMap<K, V> {
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<ReadGuard<K, V>>
    where
        K: Borrow<Q>,
        Q: Hash + PartialEq,
    {
        self.map
            .get(key)
            .filter(|guard| {
                if self.auto_clean {
                    guard.expired_at > SystemTime::now()
                } else {
                    true
                }
            })
            .map(ReadGuard)
    }

    pub fn get_mut<Q: ?Sized>(&self, key: &Q) -> Option<WriteGuard<K, V>>
    where
        K: Borrow<Q>,
        Q: Hash + PartialEq,
    {
        self.map
            .get_mut(key)
            .filter(|guard| {
                if self.auto_clean {
                    guard.expired_at > SystemTime::now()
                } else {
                    true
                }
            })
            .map(WriteGuard)
    }

    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + PartialEq,
    {
        if self.auto_clean {
            if let Some(guard) = self.map.get(key) {
                if guard.expired_at > SystemTime::now() {
                    true
                } else {
                    self.remove(key);
                    false
                }
            } else {
                false
            }
        } else {
            self.map.contains_key(key)
        }
    }

    pub fn insert(&self, key: K, val: V, expired_at: SystemTime) -> Option<(V, SystemTime)> {
        self.map
            .insert(key, Entry { expired_at, data: val })
            .filter(|entry| {
                if self.auto_clean {
                    entry.expired_at > SystemTime::now()
                } else {
                    true
                }
            })
            .map(|entry| (entry.data, entry.expired_at))
    }

    pub fn alter(&self, key: K, f: impl FnOnce(Option<(V, SystemTime)>) -> Option<(V, SystemTime)>) {
        self.map.alter(key, move |v| {
            let result = if let Some(v) = v {
                if self.auto_clean {
                    if v.expired_at > SystemTime::now() {
                        f(Some((v.data, v.expired_at)))
                    } else {
                        f(None)
                    }
                } else {
                    f(Some((v.data, v.expired_at)))
                }
            } else {
                f(None)
            };
            result.map(|(data, expired_at)| Entry { expired_at, data })
        })
    }

    pub fn remove<Q: ?Sized>(&self, key: &Q) -> Option<(V, SystemTime)>
    where
        K: Borrow<Q>,
        Q: Hash + PartialEq,
    {
        self.map.remove(key).map(|v| (v.data, v.expired_at))
    }

    pub fn from_persistent(entries: Vec<PersistentEntry<K, V>>, auto_clean: bool) -> Self {
        let map = Self::with_capacity(entries.len(), auto_clean);
        entries.into_iter().for_each(|entry| {
            map.insert(entry.key, entry.value, entry.expired_at);
        });
        map
    }
}

impl<K: PartialEq + Hash, V: Clone> CacheMap<K, V> {
    pub fn get_or_insert(&self, key: K, f: impl FnOnce() -> Option<(V, SystemTime)>) -> Option<(V, SystemTime)> {
        if let Some(cache_entry) = self.get(&key) {
            if !self.auto_clean || cache_entry.expired_at() <= SystemTime::now() {
                return Some((cache_entry.data().to_owned(), cache_entry.expired_at()));
            }
        }
        let mut result: Option<(V, SystemTime)> = None;
        self.alter(key, |cache_entry| match cache_entry {
            Some((data, expired_at)) => {
                result = Some((data, expired_at));
                result.to_owned()
            }
            None => {
                result = f();
                result.to_owned()
            }
        });
        result
    }

    pub fn try_get_or_insert<E>(
        &self,
        key: K,
        f: impl FnOnce() -> Result<Option<(V, SystemTime)>, E>,
    ) -> Result<Option<(V, SystemTime)>, E> {
        if let Some(cache_entry) = self.get(&key) {
            if !self.auto_clean || cache_entry.expired_at() <= SystemTime::now() {
                return Ok(Some((cache_entry.data().to_owned(), cache_entry.expired_at())));
            }
        }
        let mut result: Result<Option<(V, SystemTime)>, E> = Ok(None);
        self.alter(key, |cache_entry| match cache_entry {
            Some((data, expired_at)) => {
                result = Ok(Some((data.to_owned(), expired_at)));
                Some((data, expired_at))
            }
            None => match f() {
                Ok(r) => {
                    result = Ok(r.to_owned());
                    r
                }
                Err(err) => {
                    result = Err(err);
                    None
                }
            },
        });
        result
    }
}

impl<K, V> IntoIterator for CacheMap<K, V> {
    type Item = (K, V, SystemTime);
    type IntoIter = IntoIter<K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.map.into_iter())
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V, SystemTime);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k, v.data, v.expired_at))
    }
}

impl<'a, K, V> ReadGuard<'a, K, V> {
    #[inline]
    pub fn data(&self) -> &V {
        &self.0.data
    }

    #[inline]
    pub fn expired_at(&self) -> SystemTime {
        self.0.expired_at
    }
}

impl<'a, K, V> Deref for ReadGuard<'a, K, V> {
    type Target = V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<'a, K, V> WriteGuard<'a, K, V> {
    #[inline]
    pub fn data(&mut self) -> &mut V {
        &mut self.0.data
    }

    #[inline]
    pub fn expired_at(&mut self) -> &mut SystemTime {
        &mut self.0.expired_at
    }
}

impl<'a, K, V> Deref for WriteGuard<'a, K, V> {
    type Target = V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<'a, K, V> DerefMut for WriteGuard<'a, K, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut V {
        &mut self.0.data
    }
}
