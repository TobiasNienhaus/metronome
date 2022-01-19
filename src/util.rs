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
