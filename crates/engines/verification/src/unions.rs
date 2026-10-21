/// Disjoint sets of pieces that copper connects.
#[derive(Debug, Clone)]
pub(crate) struct Unions {
    parent: Vec<usize>,
}

impl Unions {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    fn root(&mut self, mut item: usize) -> usize {
        while self.parent[item] != item {
            self.parent[item] = self.parent[self.parent[item]];
            item = self.parent[item];
        }
        item
    }

    pub(crate) fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        self.parent[a.max(b)] = a.min(b);
    }

    pub(crate) fn same(&mut self, a: usize, b: usize) -> bool {
        self.root(a) == self.root(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joining_is_transitive() {
        let mut unions = Unions::new(4);
        unions.join(0, 1);
        unions.join(2, 1);
        assert!(unions.same(0, 2));
        assert!(!unions.same(0, 3));
    }
}
