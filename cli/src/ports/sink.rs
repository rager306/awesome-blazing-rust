//! Sink port: commit generated artifacts to the outside world.

pub trait Sink {
    fn write(&self, path: &str, content: &str);
}
