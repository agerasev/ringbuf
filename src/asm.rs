use crate::{Consumer, HeapRb, Producer};

pub fn push(prod: &mut Producer<i32, &HeapRb<i32>>, x: i32) -> Result<(), i32> {
    prod.push(x)
}

pub fn pop(cons: &mut Consumer<i32, &HeapRb<i32>>) -> Option<i32> {
    cons.pop()
}
