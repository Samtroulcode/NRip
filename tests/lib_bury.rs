use proptest::prelude::*;

proptest! {
    #[test]
    fn unique_name_is_never_empty(base in ".*") {
        // Suppose que unique_name(base) → "base-<ts>" ou "<uuid>" mais jamais vide
        let name = nrip::graveyard::unique_name(&base);
        prop_assert!(!name.is_empty());
    }
}
