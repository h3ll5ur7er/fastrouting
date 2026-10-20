//! Memory a search reuses from one run to the next without clearing it.

/// Entries per tile: enough that neighbouring states of a search share tiles,
/// few enough that a long, thin search holds little more than it visits.
const TILE: usize = 1024;
const UNMAPPED: u32 = u32::MAX;

/// An array that forgets all its values in O(1): entries written before the
/// last [`reset`](Self::reset) read as unset. It is held in tiles that a run
/// maps on its first write to them, from a pool kept between runs: so memory
/// follows what one search touches, not the size of its window, and reading
/// where nothing was written touches no memory at all.
#[derive(Debug, Default)]
pub struct Stamped<T> {
    /// For each tile of the array, the pool tile it is mapped to in this run.
    mapping: Vec<u32>,
    /// The tiles of the array mapped in this run, in the order of their pool tiles.
    mapped: Vec<usize>,
    /// The pool, tile after tile; flat, so a read follows one index, not a pointer.
    stamps: Vec<u32>,
    values: Vec<T>,
    generation: u32,
}

impl<T: Copy + Default> Stamped<T> {
    pub fn reset(&mut self, len: usize) {
        for &tile in &self.mapped {
            self.mapping[tile] = UNMAPPED;
        }
        self.mapped.clear();
        let tiles = len.div_ceil(TILE);
        if self.mapping.len() < tiles {
            self.mapping.resize(tiles, UNMAPPED);
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            // After 2³² runs old stamps would look current again.
            self.stamps.fill(0);
            self.generation = 1;
        }
    }

    pub fn get(&self, index: usize) -> Option<T> {
        let tile = self.mapping[index / TILE];
        if tile == UNMAPPED {
            return None;
        }
        let at = tile as usize * TILE + index % TILE;
        (self.stamps[at] == self.generation).then(|| self.values[at])
    }

    pub fn set(&mut self, index: usize, value: T) {
        let at = self.pool_tile(index / TILE) * TILE + index % TILE;
        self.stamps[at] = self.generation;
        self.values[at] = value;
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

    /// The pool tile that holds `tile` in this run, mapping one if it has none.
    fn pool_tile(&mut self, tile: usize) -> usize {
        if self.mapping[tile] == UNMAPPED {
            let next = self.mapped.len();
            if next == self.tiles_held() {
                self.stamps.resize(self.stamps.len() + TILE, 0);
                self.values.resize(self.values.len() + TILE, T::default());
            }
            self.mapping[tile] = u32::try_from(next).unwrap_or(UNMAPPED - 1);
            self.mapped.push(tile);
        }
        self.mapping[tile] as usize
    }

    fn tiles_held(&self) -> usize {
        self.stamps.len() / TILE
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
    fn values_of_a_tile_reused_by_a_later_run_are_forgotten() {
        let mut stamped = Stamped::<i64>::default();
        stamped.reset(4 * TILE);
        stamped.set(5, 7);
        stamped.reset(4 * TILE);
        // The tile that held index 5 now serves another part of the array.
        stamped.set(3 * TILE, 1);
        assert_eq!(stamped.get(3 * TILE + 5), None);
        assert_eq!(stamped.get(5), None);
    }

    #[test]
    fn holds_only_the_tiles_one_run_writes_to() {
        let mut stamped = Stamped::<i64>::default();
        stamped.reset(1000 * TILE);
        stamped.set(0, 1);
        stamped.set(999 * TILE, 2);
        stamped.reset(1000 * TILE);
        stamped.set(500 * TILE, 3);
        assert_eq!(stamped.tiles_held(), 2);
    }

    #[test]
    fn values_stay_forgotten_when_the_run_counter_wraps() {
        let mut stamped = Stamped::<i64>::default();
        stamped.reset(2);
        stamped.set(0, 7);
        stamped.generation = u32::MAX;
        stamped.reset(2);
        stamped.set(1, 1);
        assert_eq!(stamped.get(0), None);
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
