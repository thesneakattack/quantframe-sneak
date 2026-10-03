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

#[cfg(test)]
mod tests {
    use super::{complete_copies, WFInvSetMember};

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
}
