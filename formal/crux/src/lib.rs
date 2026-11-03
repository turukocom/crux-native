pub mod tcb_rights;
use tcb_rights::Rights;

#[cfg_attr(crux, crux::test)]
fn test_add_one() {
    assert_eq!(1u32 + 1, 2);
}

#[cfg_attr(crux, crux::test)]
fn test_tcb_rights() {
    assert_eq!(Rights::empty().contains(Rights::empty()), true);
    assert_eq!(Rights::READ.union(Rights::WRITE).contains(Rights::READ), true);
    assert_eq!(Rights::ALL.contains(Rights::MANAGE), true);
    assert_eq!(Rights::READ.is_empty(), false);
    assert_eq!(Rights::masked(Rights(0x1ff)).0, 0xff);
}
