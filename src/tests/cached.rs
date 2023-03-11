use crate::HeapRb;

#[test]
fn producer() {
    let (mut prod, mut cons) = HeapRb::<i32>::new(2).split();
    prod.push(0).unwrap();
    assert!(cons.iter().cloned().eq(0..1));

    {
        let mut post_prod = prod.postponed();

        post_prod.push(1).unwrap();
        assert!(cons.iter().cloned().eq(0..1));
        assert_eq!(cons.len(), 1);
        assert_eq!(post_prod.len(), 2);

        assert_eq!(cons.pop().unwrap(), 0);
        assert!(cons.pop().is_none());
        assert_eq!(cons.len(), 0);
        assert_eq!(post_prod.len(), 2);

        post_prod.sync();
        assert!(cons.iter().cloned().eq(1..2));
        assert_eq!(cons.len(), 1);
        assert_eq!(post_prod.len(), 1);

        post_prod.push(2).unwrap();
        assert!(cons.iter().cloned().eq(1..2));
        assert_eq!(cons.len(), 1);
        assert_eq!(post_prod.len(), 2);
    }

    assert!(cons.iter().cloned().eq(1..3));
    assert_eq!(cons.len(), 2);
    assert_eq!(prod.len(), 2);
}

#[test]
fn discard() {
    let (mut prod, cons) = HeapRb::<i32>::new(10).split();
    prod.push(0).unwrap();
    assert!(cons.iter().cloned().eq(0..1));

    {
        let mut post_prod = prod.postponed();

        post_prod.push(1).unwrap();
        assert_eq!(cons.len(), 1);
        assert_eq!(post_prod.len(), 2);

        post_prod.sync();
        assert!(cons.iter().cloned().eq(0..2));
        assert_eq!(cons.len(), 2);
        assert_eq!(post_prod.len(), 2);

        post_prod.push(3).unwrap();
        assert_eq!(cons.len(), 2);
        assert_eq!(post_prod.len(), 3);

        post_prod.discard();
        assert_eq!(cons.len(), 2);
        assert_eq!(post_prod.len(), 2);

        post_prod.push(2).unwrap();
        assert_eq!(cons.len(), 2);
        assert_eq!(post_prod.len(), 3);
    }

    assert!(cons.iter().cloned().eq(0..3));
    assert_eq!(cons.len(), 3);
    assert_eq!(prod.len(), 3);
}

#[test]
fn consumer() {
    let (mut prod, mut cons) = HeapRb::<i32>::new(10).split();
    prod.push(0).unwrap();
    prod.push(1).unwrap();
    assert!(cons.iter().cloned().eq(0..2));

    {
        let mut post_cons = cons.postponed();

        assert_eq!(post_cons.pop().unwrap(), 0);
        assert!(post_cons.iter().cloned().eq(1..2));
        assert_eq!(post_cons.len(), 1);
        assert_eq!(prod.len(), 2);

        prod.push(2).unwrap();
        assert!(post_cons.iter().cloned().eq(1..2));
        assert_eq!(post_cons.len(), 1);
        assert_eq!(prod.len(), 3);

        post_cons.sync();
        assert!(post_cons.iter().cloned().eq(1..3));
        assert_eq!(post_cons.len(), 2);
        assert_eq!(prod.len(), 2);

        assert_eq!(post_cons.pop().unwrap(), 1);
        assert!(post_cons.iter().cloned().eq(2..3));
        assert_eq!(post_cons.len(), 1);
        assert_eq!(prod.len(), 2);
    }

    assert!(cons.iter().cloned().eq(2..3));
    assert_eq!(cons.len(), 1);
    assert_eq!(prod.len(), 1);
}
