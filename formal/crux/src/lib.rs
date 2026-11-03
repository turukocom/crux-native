extern crate crucible;
use crucible::Symbolic;

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


#[cfg_attr(crux, crux::test)]
fn test_rights_exhaustive_8bit() {
    for x in 0u32..256 {
        let r = Rights::from_bits(x);
        assert_eq!(r.is_empty(), x == 0);
        assert_eq!(r.union(Rights::READ).contains(Rights::READ), true);
    }
    // RightsAmplification: any bit outside 0xff must be rejected by install.
    for x in 0u32..256 {
        let mut caps = CapTable::new();
        let (obj, gen) = caps.create_object(1).unwrap();
        let bad = Rights::from_bits(x | 0x100);
        let cap = Cap { object: obj, generation: gen, rights: bad };
        assert!(caps.install(0, cap).is_err());
    }
}


#[cfg_attr(crux, crux::test)]
fn test_symbolic_rights_contains() {
    let a = u32::symbolic("a");
    let b = u32::symbolic("b");
    let ra = Rights::from_bits(a);
    let rb = Rights::from_bits(b);
    assert_eq!(ra.contains(rb), (a & b) == b);
}
