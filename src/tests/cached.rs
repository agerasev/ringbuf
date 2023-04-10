use crate::{storage::Static, traits::*, LocalRb};

#[test]
fn producer() {
    let mut rb = LocalRb::<Static<i32, 2>>::default();
    let (mut prod, mut cons) = rb.split_ref();
    prod.try_push(0).unwrap();
    assert!(cons.iter().cloned().eq(0..1));

    {
        let mut cached_prod = prod.cached();

        cached_prod.try_push(1).unwrap();
        assert!(cons.iter().cloned().eq(0..1));
        assert_eq!(cons.occupied_len(), 1);
        assert_eq!(cached_prod.occupied_len(), 2);

        assert_eq!(cons.try_pop().unwrap(), 0);
        assert!(cons.try_pop().is_none());
        assert_eq!(cons.occupied_len(), 0);
        assert_eq!(cached_prod.occupied_len(), 2);

        cached_prod.sync();
        assert!(cons.iter().cloned().eq(1..2));
        assert_eq!(cons.occupied_len(), 1);
        assert_eq!(cached_prod.occupied_len(), 1);

        cached_prod.try_push(2).unwrap();
        assert!(cons.iter().cloned().eq(1..2));
        assert_eq!(cons.occupied_len(), 1);
        assert_eq!(cached_prod.occupied_len(), 2);
    }

    assert!(cons.iter().cloned().eq(1..3));
    assert_eq!(cons.occupied_len(), 2);
    assert_eq!(prod.occupied_len(), 2);
}

#[test]
fn discard() {
    let mut rb = LocalRb::<Static<i32, 10>>::default();
    let (mut prod, cons) = rb.split_ref();
    prod.try_push(0).unwrap();
    assert!(cons.iter().cloned().eq(0..1));

    {
        let mut cached_prod = prod.cached();

        cached_prod.try_push(1).unwrap();
        assert_eq!(cons.occupied_len(), 1);
        assert_eq!(cached_prod.occupied_len(), 2);

        cached_prod.sync();
        assert!(cons.iter().cloned().eq(0..2));
        assert_eq!(cons.occupied_len(), 2);
        assert_eq!(cached_prod.occupied_len(), 2);

        cached_prod.try_push(3).unwrap();
        assert_eq!(cons.occupied_len(), 2);
        assert_eq!(cached_prod.occupied_len(), 3);

        cached_prod.discard();
        assert_eq!(cons.occupied_len(), 2);
        assert_eq!(cached_prod.occupied_len(), 2);

        cached_prod.try_push(2).unwrap();
        assert_eq!(cons.occupied_len(), 2);
        assert_eq!(cached_prod.occupied_len(), 3);
    }

    assert!(cons.iter().cloned().eq(0..3));
    assert_eq!(cons.occupied_len(), 3);
    assert_eq!(prod.occupied_len(), 3);
}

#[test]
fn consumer() {
    let mut rb = LocalRb::<Static<i32, 10>>::default();
    let (mut prod, mut cons) = rb.split_ref();
    prod.try_push(0).unwrap();
    prod.try_push(1).unwrap();
    assert!(cons.iter().cloned().eq(0..2));

    {
        let mut cached_cons = cons.cached();

        assert_eq!(cached_cons.try_pop().unwrap(), 0);
        assert!(cached_cons.iter().cloned().eq(1..2));
        assert_eq!(cached_cons.occupied_len(), 1);
        assert_eq!(prod.occupied_len(), 2);

        prod.try_push(2).unwrap();
        assert!(cached_cons.iter().cloned().eq(1..2));
        assert_eq!(cached_cons.occupied_len(), 1);
        assert_eq!(prod.occupied_len(), 3);

        cached_cons.sync();
        assert!(cached_cons.iter().cloned().eq(1..3));
        assert_eq!(cached_cons.occupied_len(), 2);
        assert_eq!(prod.occupied_len(), 2);

        assert_eq!(cached_cons.try_pop().unwrap(), 1);
        assert!(cached_cons.iter().cloned().eq(2..3));
        assert_eq!(cached_cons.occupied_len(), 1);
        assert_eq!(prod.occupied_len(), 2);
    }

    assert!(cons.iter().cloned().eq(2..3));
    assert_eq!(cons.occupied_len(), 1);
    assert_eq!(prod.occupied_len(), 1);
}
