/// Whether a tradable item is an arcane enhancement.
///
/// Decided by the cache's tag, the same authority tradability uses. The tag
/// matters more than the name here: `Virtuos Strike` and `Pax Soar` are
/// arcanes and say nothing about it, while `Arcane Thrak Helmet` is a cosmetic
/// skin carrying `arcane_helmet`. Matching on the word "Arcane" would get both
/// of those wrong.
///
/// A handful of arcanes are tagged `mod` as well. The arcane tag wins, because
/// an arcane is what the player is deciding about.
pub fn is_arcane(tags: &[String]) -> bool {
    tags.iter().any(|tag| tag == "arcane_enhancement")
}

#[cfg(test)]
mod tests {
    use super::is_arcane;

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn recognises_an_arcane_by_its_tag() {
        assert!(is_arcane(&tags(&["arcane_enhancement", "rare"])));
    }

    /// Operator and focus arcanes carry no "Arcane" in their name at all, so
    /// anything matching on the name would miss them.
    #[test]
    fn recognises_an_arcane_whose_name_does_not_say_so() {
        assert!(is_arcane(&tags(&["arcane_enhancement", "uncommon"])));
    }

    /// `Arcane Thrak Helmet` is a cosmetic skin, not an arcane, and putting
    /// cosmetics in the arcanes tab would bury the items worth listing.
    #[test]
    fn does_not_mistake_an_arcane_helmet_for_an_arcane() {
        assert!(!is_arcane(&tags(&["arcane_helmet", "skin"])));
    }

    #[test]
    fn does_not_mistake_a_plain_mod_for_an_arcane() {
        assert!(!is_arcane(&tags(&["mod", "rare"])));
        assert!(!is_arcane(&[]));
    }
}
