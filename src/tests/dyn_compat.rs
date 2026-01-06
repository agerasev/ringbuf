use crate::traits::{Consumer, Producer};

#[test]
fn producer_should_dyn_compatible() {
    fn _asdf(_a: &dyn Producer<Item = f32>) {}
}

#[test]
fn consumer_should_dyn_compatible() {
    fn _asdf(_a: &dyn Consumer<Item = f32>) {}
}
