use chainx_primitives::Hash;

pub(crate) struct Cache<T> {
    pub hash: Hash,
    pub data: T,
}

static mut OPEN_CACHE: bool = true;

pub fn set_cache_flag(open: bool) {
    unsafe {
        OPEN_CACHE = open;
    }
}

#[inline]
pub(crate) fn get_cache_flag() -> bool {
    unsafe { OPEN_CACHE }
}

macro_rules! lru_cache {
    ($VT: ty; $hash:ident; $sel:ident $code_block:block) => {
        {
            let lru_cache_u32_key = 0u32;
            lru_cache!(u32, $VT, size=1; key=lru_cache_u32_key; $hash; $sel $code_block)
        }
    };

    ($KT:ty, $VT: ty, size=$size:expr; key=$key:ident; $hash:ident; $sel:ident $code_block:block) => {
        {
            let not_use_cache = $hash.is_some() || !$crate::chainx::cache::get_cache_flag();
            let data = if !not_use_cache {
                lazy_static::lazy_static! {
                    static ref CACHE: std::sync::Mutex<lru::LruCache<$KT, $crate::chainx::cache::Cache<$VT>>> = std::sync::Mutex::new(lru::LruCache::new($size));
                }
                let mut cache = match CACHE.lock() {
                    Ok(i) => i,
                    Err(_) => return Err(Error::CacheErr.into()),
                };
                // do cache
                let best_hash = $sel.client.info().chain.best_hash;
                if let Some(item) = cache.get(&$key) {
                    if item.hash == best_hash {
                        // hit cache
                        Ok(item.data.clone())
                    } else {
                        Err(Some((cache, best_hash)))
                    }
                } else {
                    Err(Some((cache, best_hash)))
                }
            } else {
                Err(None)
            };
            match data {
                Ok(result) => result,
                Err(op) => {
                    let code_result = $code_block;
                    match op {
                        Some((mut c, best_hash)) => {
                            c.put(
                                $key.clone(),
                                $crate::chainx::cache::Cache {
                                    hash: best_hash,
                                    data: code_result.clone(),
                                },
                            );
                        },
                        None => {}
                    }
                    code_result
                }
            }
        }
    };
}
