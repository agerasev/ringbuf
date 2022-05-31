use crate::{Consumer, Producer};

pub fn push(prod: &mut Producer<i32>, x: i32) -> Result<(), i32> {
    prod.push(x)
}

pub fn pop(cons: &mut Consumer<i32>) -> Option<i32> {
    cons.pop()
}
