use super::*;

#[test]
fn test_hashset_unwrap() {
    let hashset = Arc::new(Mutex::new(HashSet::from([1, 2, 3])));
    let raw_hashset = hashset_unwrap(&hashset);
    assert_eq!(HashSet::from([1, 2, 3]), raw_hashset);
}

#[test]
fn test_hashset_pop() {
    let mut hashset = Arc::new(Mutex::new(HashSet::from([1, 2, 3])));
    let popped = hashset_pop(&mut hashset);
    let current_hashset = hashset_unwrap(&hashset);
    assert!(popped <= 3);
    assert_eq!(2, current_hashset.len())
}

#[test]
fn test_hashset_push() {
    let hashset = Arc::new(Mutex::new(HashSet::from([1, 2, 3])));
    hashset_push(&hashset, 4);
    hashset_push(&hashset, 2);
    let current = hashset_unwrap(&hashset);
    assert_eq!(4, current.len());
}

#[test]
fn test_hashset_exists() {
    let hashset = Arc::new(Mutex::new(HashSet::from([1, 2, 3])));
    assert!(hashset_exists(&hashset, &3));
    assert!(!hashset_exists(&hashset, &100));
}

#[test]
fn test_hashset_append() {
    let hashset = Arc::new(Mutex::new(HashSet::from([1])));
    hashset_append(&hashset, vec![2, 3, 4, 1]);
    let current = hashset_unwrap(&hashset);
    assert_eq!(4, current.len());
}
