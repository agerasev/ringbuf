//! Executable cursor contract, independent of the unsafe storage implementation.

#[derive(Clone, Copy, Debug)]
struct Model {
    capacity: usize,
    released: usize,
    claimed: usize,
    published: usize,
}

impl Model {
    fn check(self) {
        assert!(self.released <= self.claimed);
        assert!(self.claimed <= self.published);
        assert!(self.published - self.released <= self.capacity);
        let modulus = 2 * self.capacity;
        let distance = |end: usize, start: usize| (end % modulus + modulus - start % modulus) % modulus;
        assert_eq!(distance(self.published, self.released), self.published - self.released);
        assert_eq!(distance(self.published, self.claimed), self.published - self.claimed);
    }
}

#[test]
fn commit_preserves_unread_claims_and_finalization_returns_them() {
    for capacity in 1..=8 {
        for start in 0..4 * capacity {
            for taken in 0..=capacity {
                let mut rb = Model {
                    capacity,
                    released: start,
                    claimed: start,
                    published: start + capacity,
                };
                rb.claimed = rb.published; // fetch transfers ownership of every queued item
                rb.check();
                rb.released = start + taken; // commit releases only the consumed prefix
                rb.check();
                assert_eq!(rb.claimed - rb.released, capacity - taken);
                rb.claimed = rb.released; // finalization returns the untouched suffix
                rb.check();
                assert_eq!(rb.published - rb.claimed, capacity - taken);
            }
        }
    }
}

#[test]
fn forgotten_claim_is_abandoned_before_subsequent_read() {
    for capacity in 1..=8 {
        for start in 0..4 * capacity {
            for claimed in 0..=capacity {
                let mut rb = Model {
                    capacity,
                    released: start,
                    claimed: start + claimed,
                    published: start + capacity,
                };
                rb.check();
                rb.released = rb.claimed; // never drop or inspect the abandoned prefix
                rb.check();
                assert_eq!(rb.published - rb.claimed, capacity - claimed);
                rb.published += claimed; // producer can reuse the complete abandoned range
                rb.check();
            }
        }
    }
}

#[test]
fn fetch_extends_an_active_claim_without_releasing_it() {
    for capacity in 2..=8 {
        let mut rb = Model {
            capacity,
            released: 0,
            claimed: 1,
            published: 1,
        };
        rb.published = capacity;
        let additional = rb.published - rb.claimed;
        rb.claimed = rb.published;
        rb.check();
        assert_eq!(additional, capacity - 1);
        assert_eq!(rb.released, 0);
        // Returning a consumed value moves only the endpoint-local cursor. The
        // RB still excludes the entire claim from its own destruction range.
        let mut cursor = 1;
        cursor -= 1;
        rb.claimed = cursor; // drop restores every unread/restored item
        rb.released = cursor;
        rb.check();
        assert_eq!(rb.published - rb.claimed, capacity);
    }
}
