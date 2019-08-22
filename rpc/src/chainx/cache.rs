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
        let lru_cache_u32_key = 0u32;
        lru_cache!(u32, $VT, size=1; key=lru_cache_u32_key; $hash; $sel $code_block)
    };

    ($KT:ty, $VT: ty, size=$size:expr; key=$key:ident; $hash:ident; $sel:ident $code_block:block) => {
        if $hash.is_some() || !$crate::chainx::cache::get_cache_flag() {
            return $code_block;
        }
        // do cache
        let best_hash = $sel.client.info()?.chain.best_hash;
        lazy_static::lazy_static! {
            static ref CACHE: std::sync::Mutex<lru::LruCache<$KT, $crate::chainx::cache::Cache<$VT>>> = std::sync::Mutex::new(lru::LruCache::new($size));
        }
        let mut cache = match CACHE.lock() {
            Ok(i) => i,
            Err(_) => return Err(ErrorKind::CacheErr.into()),
        };
        if let Some(item) = cache.get(&$key) {
            // hit cache
            if item.hash == best_hash {
                return Ok(item.data.clone());
            }
        }

        // otherwise, do `code_block`, set result into cache, return it
        let r = $code_block;
        if let Ok(ref item) = r {
            cache.put(
                $key,
                $crate::chainx::cache::Cache {
                    hash: best_hash,
                    data: item.clone(),
                },
            );
        }
        return r;
    };
}
