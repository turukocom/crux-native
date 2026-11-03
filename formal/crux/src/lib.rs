fn add_one(x: u32) -> u32 { x + 1 }

#[cfg_attr(crux, crux::test)]
fn test_add_one() {
    assert_eq!(add_one(1), 2);
}
