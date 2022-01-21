#[derive(Debug)]
pub struct SyncWrapper<T> {
    inner: T,
}

unsafe impl<T> Sync for SyncWrapper<T> {}

impl<T> SyncWrapper<T>
where
    T: Clone,
{
    pub fn get(&self) -> T {
        self.inner.clone()
    }

    pub fn new(inner: &T) -> SyncWrapper<T> {
        SyncWrapper {
            inner: inner.clone(),
        }
    }
}

pub fn busy_sleep(ns: u128) {
    busy_sleep_from(std::time::Instant::now(), ns);
}

pub fn busy_sleep_from(start: std::time::Instant, ns: u128) {
    while std::time::Instant::now().duration_since(start).as_nanos() < ns {}
}

pub const fn bpm_to_ns(bpm: u128) -> u128 {
    (60000 * 1000000) / bpm
}
