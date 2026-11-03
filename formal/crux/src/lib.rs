#[path = "tcb_cap.rs"]
pub mod cap;
use cap::{Cap, CapTable, Rights};

#[cfg_attr(crux, crux::test)]
fn test_add_one() {
    assert_eq!(1u32 + 1, 2);
}

#[cfg_attr(crux, crux::test)]
fn test_actual_cap_smoke() {
    let mut caps = CapTable::new();
    let (obj, gen) = caps.create_object(1).unwrap();
    assert_eq!(obj, 0);
    assert_eq!(gen, 1);
    caps.install(0, Cap { object: obj, generation: gen, rights: Rights::READ })
        .unwrap();
    let c = caps.lookup(0, Rights::READ).unwrap();
    assert_eq!(c.object, 0);
    assert_eq!(c.generation, 1);
    assert_eq!(Rights::READ.union(Rights::WRITE).contains(Rights::READ), true);
}

#[cfg_attr(crux, crux::test)]
fn test_actual_cap_failures() {
    let mut caps = CapTable::new();
    // lookup empty
    assert!(caps.lookup(0, Rights::READ).is_err());
    // install invalid object
    let bad = Cap { object: 63, generation: 1, rights: Rights::READ };
    assert!(caps.install(0, bad).is_err());
    // create objects until all alive, then ObjectLimit
    let mut gen0 = 0u16;
    for _ in 0..64 {
        let (_, g) = caps.create_object(1).unwrap();
        gen0 = g;
    }
    assert_eq!(gen0, 1);
    assert!(caps.create_object(1).is_err());
}
