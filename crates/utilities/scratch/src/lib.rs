//! Memory a search reuses from one run to the next without clearing it.

/// A dense array that forgets all its values in O(1): entries written before
/// the last [`reset`](Self::reset) read as unset. One allocation then serves
/// every search, however many connections are routed.
#[derive(Debug, Default)]
pub struct Stamped<T> {
    values: Vec<T>,
    stamps: Vec<u32>,
    generation: u32,
}

impl<T: Copy + Default> Stamped<T> {
    pub fn reset(&mut self, len: usize) {
        if self.stamps.len() < len {
            self.values.resize(len, T::default());
            self.stamps.resize(len, 0);
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            // After 2³² resets old stamps would look current again.
            self.stamps.fill(0);
            self.generation = 1;
        }
    }

    pub fn get(&self, index: usize) -> Option<T> {
        (self.stamps[index] == self.generation).then(|| self.values[index])
    }

    pub fn set(&mut self, index: usize, value: T) {
        self.values[index] = value;
        self.stamps[index] = self.generation;
    }

    pub fn get_or_insert_with(&mut self, index: usize, value: impl FnOnce() -> T) -> T {
        match self.get(index) {
            Some(known) => known,
            None => {
                let value = value();
                self.set(index, value);
                value
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_forgotten_on_reset() {
        let mut stamped = Stamped::<i64>::default();
        stamped.reset(4);
        stamped.set(2, 7);
        assert_eq!(stamped.get(2), Some(7));
        stamped.reset(4);
        assert_eq!(stamped.get(2), None);
    }

    #[test]
    fn growing_keeps_working() {
        let mut stamped = Stamped::<u8>::default();
        stamped.reset(1);
        stamped.reset(10);
        stamped.set(9, 1);
        assert_eq!(stamped.get(9), Some(1));
    }
}
