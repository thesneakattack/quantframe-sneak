use serde::{Deserialize, Serialize};

use crate::wf_inventory::item_base::WFInvItemBase;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvSetMember {
    pub unique_name: String,
    pub name: String,
    pub have: i64,
    pub required: i64,
    pub is_main_blueprint: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvSet {
    #[serde(flatten)]
    pub base: WFInvItemBase,
    pub members: Vec<WFInvSetMember>,
    pub complete_copies: i64,
    pub owned_members: i64,
    pub total_members: i64,
}

/// How many whole copies of the set the player holds.
///
/// Integer division, because 57 tradable ingredients across the real set list
/// require more than one copy of a part. A member whose `required` is zero or
/// negative is treated as unsatisfiable rather than dividing by zero, and a
/// set with no members yields zero rather than panicking on an empty `min()`.
pub fn complete_copies(members: &[WFInvSetMember]) -> i64 {
    members
        .iter()
        .map(|member| {
            if member.required <= 0 {
                0
            } else {
                member.have / member.required
            }
        })
        .min()
        .unwrap_or(0)
}

/// How many of the set's members the player actually has enough of.
///
/// Presence is not enough: a member held 1 of 2 is still missing. Counting
/// presence makes a set short on quantity render "Missing 0" while offering no
/// cart action, which reads as "nothing is wrong" and "I refuse" at once.
pub fn satisfied_members(members: &[WFInvSetMember]) -> i64 {
    members
        .iter()
        .filter(|member| member.required > 0 && member.have >= member.required)
        .count() as i64
}

#[cfg(test)]
mod tests {
    use super::{complete_copies, satisfied_members, WFInvSetMember};

    fn member(have: i64, required: i64) -> WFInvSetMember {
        WFInvSetMember {
            unique_name: "/X".to_string(),
            name: "X".to_string(),
            have,
            required,
            is_main_blueprint: false,
        }
    }

    #[test]
    fn counts_whole_copies_only() {
        let members = vec![member(3, 1), member(2, 1), member(5, 1)];
        assert_eq!(complete_copies(&members), 2);
    }

    #[test]
    fn divides_by_the_required_quantity() {
        let members = vec![member(12, 6), member(2, 1)];
        assert_eq!(complete_copies(&members), 2);
    }

    #[test]
    fn reports_zero_when_a_member_is_missing() {
        let members = vec![member(3, 1), member(0, 1)];
        assert_eq!(complete_copies(&members), 0);
    }

    /// A recipe with no tradable ingredients leaves nothing to divide. min()
    /// over an empty iterator is None, and unwrapping it would panic and take
    /// the whole Sets tab down.
    #[test]
    fn reports_zero_for_a_set_with_no_members() {
        assert_eq!(complete_copies(&[]), 0);
    }

    /// A required quantity of zero would be a division by zero. Treat it as
    /// unsatisfiable rather than crashing.
    #[test]
    fn treats_a_zero_requirement_as_unsatisfiable_rather_than_panicking() {
        let members = vec![member(5, 0), member(5, 1)];
        assert_eq!(complete_copies(&members), 0);
    }

    /// A member you hold but not enough of is not a member you have. Counting
    /// mere presence makes a set short of a required quantity render
    /// "Missing 0" with no cart action - the one combination that tells the
    /// user nothing is wrong while refusing to act.
    #[test]
    fn a_member_short_of_its_required_quantity_does_not_count_as_satisfied() {
        let members = vec![member(2, 2), member(1, 2)];
        assert_eq!(satisfied_members(&members), 1);
    }

    #[test]
    fn counts_every_member_that_meets_its_requirement() {
        let members = vec![member(2, 2), member(5, 1)];
        assert_eq!(satisfied_members(&members), 2);
    }

    #[test]
    fn counts_nothing_for_an_empty_set() {
        assert_eq!(satisfied_members(&[]), 0);
    }
}
