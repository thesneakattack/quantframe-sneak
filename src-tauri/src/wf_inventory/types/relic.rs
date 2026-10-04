use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::wf_inventory::item_base::WFInvItemBase;

/// The order the game refines a relic in. Sorting by this rather than
/// alphabetically keeps Intact first and Radiant last, which is how a player
/// reads a relic: cheapest and most plentiful through to most refined.
pub const REFINEMENTS: [&str; 4] = ["intact", "exceptional", "flawless", "radiant"];

/// One refinement of a relic the player holds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvRelicRefinement {
    pub variant: String,
    pub owned: i64,
    pub price: Option<f64>,
}

/// A relic, with every refinement of it the player holds on one row.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvRelic {
    #[serde(flatten)]
    pub base: WFInvItemBase,
    pub refinements: Vec<WFInvRelicRefinement>,
    pub total_owned: i64,
}

/// Where a refinement sorts. Anything unrecognised goes last rather than
/// being dropped, so a relic type the game adds later still renders.
fn refinement_order(variant: &str) -> usize {
    REFINEMENTS
        .iter()
        .position(|r| *r == variant)
        .unwrap_or(REFINEMENTS.len())
}

/// Whether a tradable item is a void relic.
///
/// Decided by the cache's tag, the same authority tradability itself uses.
/// Classifying by `uniqueName` prefix instead would be a guess that breaks on
/// the next relic family the game adds.
pub fn is_relic(tags: &[String]) -> bool {
    tags.iter().any(|tag| tag == "relic")
}

/// What a relic is worth at its best refinement.
///
/// A grouped row needs one figure to sort and filter by, and the useful one is
/// the most you could get for it: a single Radiant is the reason to keep a
/// relic whose Intact copies are near worthless. Unpriced refinements are
/// skipped rather than counted as zero, so one known price still values the
/// row and a relic with none stays honestly unknown.
pub fn best_price(relic: &WFInvRelic) -> Option<f64> {
    relic
        .refinements
        .iter()
        .filter_map(|refinement| refinement.price)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

/// Collapse per-refinement rows into one row per relic.
///
/// The snapshot emits a row per (relic, refinement) because that is how the
/// inventory stores them and how warframe.market prices them - an Intact and a
/// Radiant are different products. For reading, though, they are one thing you
/// own, and the decision being made is which refinement to sell. Grouping on
/// `wfm_url` keeps the market identity as the key, so two refinements can
/// never merge across relics.
pub fn group_refinements(rows: Vec<WFInvItemBase>) -> Vec<WFInvRelic> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, WFInvRelic> = HashMap::new();

    for row in rows {
        // An unrefined relic matches the cache item's own unique name, so
        // `variant_of` reports nothing for it. That absence means Intact.
        let variant = row
            .sub_type
            .as_ref()
            .and_then(|s| s.variant.clone())
            .unwrap_or_else(|| REFINEMENTS[0].to_string());
        let price = row
            .properties
            .get_property_value::<Option<f64>>("price", None);
        let owned = row.quantity;
        let key = row.wfm_url.clone();

        let relic = grouped.entry(key.clone()).or_insert_with(|| {
            order.push(key);
            let mut base = row.clone();
            // The row stands for the relic, not for whichever refinement was
            // seen first. Keeping that sub-type would label every row with one
            // refinement's name and let it be listed as the wrong product.
            base.sub_type = None;
            WFInvRelic {
                base,
                refinements: Vec::new(),
                total_owned: 0,
            }
        });
        relic.refinements.push(WFInvRelicRefinement {
            variant,
            owned,
            price,
        });
        relic.total_owned += owned;
    }

    order
        .into_iter()
        .filter_map(|key| grouped.remove(&key))
        .map(|mut relic| {
            relic
                .refinements
                .sort_by_key(|r| refinement_order(&r.variant));
            relic
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{best_price, group_refinements, is_relic, refinement_order, WFInvItemBase};
    use utils::SubType;

    fn tag(t: &str) -> String {
        t.to_string()
    }

    fn relic_row(
        url: &str,
        variant: Option<&str>,
        owned: i64,
        price: Option<f64>,
    ) -> WFInvItemBase {
        let mut row = WFInvItemBase {
            name: "Lith N15 Relic".to_string(),
            wfm_url: url.to_string(),
            quantity: owned,
            sub_type: variant.map(|v| SubType {
                variant: Some(v.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        row.properties.set_property_value("price", price);
        row
    }

    /// Relics are classified by the cache's tag, not by a unique-name prefix,
    /// for the same reason tradability is: the tags are the authority and a
    /// path prefix is a guess that breaks on the next item the game adds.
    #[test]
    fn recognises_a_relic_by_its_tag() {
        assert!(is_relic(&[tag("relic"), tag("lith")]));
    }

    /// A prime part is not a relic however it is tagged otherwise.
    #[test]
    fn does_not_mistake_another_tradable_item_for_a_relic() {
        assert!(!is_relic(&[tag("prime"), tag("component")]));
        assert!(!is_relic(&[]));
    }

    /// The grouped row needs one figure to sort and filter by, and the one
    /// that matters is the best you could get: a Radiant worth 20p is the
    /// reason to keep the relic, even when the Intact copies are worth 4p.
    #[test]
    fn values_a_relic_at_its_most_valuable_refinement() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 3, Some(4.0)),
            relic_row("lith_n15_relic", Some("radiant"), 1, Some(20.0)),
        ];
        assert_eq!(best_price(&group_refinements(rows)[0]), Some(20.0));
    }

    /// A relic whose refinements are all unpriced is honestly unknown, not
    /// zero, matching how every other tab treats a missing price.
    #[test]
    fn reports_no_value_when_no_refinement_is_priced() {
        let rows = vec![relic_row("lith_n15_relic", Some("intact"), 3, None)];
        assert_eq!(best_price(&group_refinements(rows)[0]), None);
    }

    /// One priced refinement among unpriced ones still gives the row a value.
    #[test]
    fn ignores_unpriced_refinements_when_valuing_a_relic() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 3, None),
            relic_row("lith_n15_relic", Some("radiant"), 1, Some(9.0)),
        ];
        assert_eq!(best_price(&group_refinements(rows)[0]), Some(9.0));
    }

    /// Four refinements of one relic are one thing the player owns, not four
    /// unrelated rows. Grouping them is the whole point of the tab: you decide
    /// which refinement to sell by comparing them side by side.
    #[test]
    fn collapses_every_refinement_of_one_relic_into_a_single_row() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 3, Some(4.0)),
            relic_row("lith_n15_relic", Some("radiant"), 1, Some(20.0)),
        ];
        let grouped = group_refinements(rows);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].refinements.len(), 2);
    }

    /// The grouped row is the relic, not one refinement of it, so it must not
    /// inherit the sub-type of whichever refinement happened to be seen first.
    /// Leaving it on renders every row as "Neo S3 Relic [Intact]" even when
    /// the player holds Radiant copies too.
    #[test]
    fn the_grouped_row_carries_no_single_refinements_sub_type() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 3, Some(4.0)),
            relic_row("lith_n15_relic", Some("radiant"), 1, Some(20.0)),
        ];
        assert!(group_refinements(rows)[0].base.sub_type.is_none());
    }

    /// The owned count on the collapsed row is every copy across refinements,
    /// or the row understates what you hold.
    #[test]
    fn totals_owned_across_refinements() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 3, Some(4.0)),
            relic_row("lith_n15_relic", Some("radiant"), 1, Some(20.0)),
        ];
        assert_eq!(group_refinements(rows)[0].total_owned, 4);
    }

    /// Different relics must not merge just because they are both relics.
    #[test]
    fn keeps_distinct_relics_apart() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 1, None),
            relic_row("axi_t2_relic", Some("intact"), 1, None),
        ];
        assert_eq!(group_refinements(rows).len(), 2);
    }

    /// `variant_of` reports no variant when the owned unique name is the
    /// item's own, and for a relic that canonical form is the unrefined one.
    /// Labelling it Intact is what keeps an unrefined relic from rendering as
    /// a blank refinement.
    #[test]
    fn treats_a_row_with_no_variant_as_intact() {
        let rows = vec![relic_row("requiem_eterna_relic", None, 2, None)];
        let grouped = group_refinements(rows);
        assert_eq!(grouped[0].refinements[0].variant, "intact");
    }

    /// Refinements read in the order the game refines them, so Intact leads
    /// and Radiant trails however the rows arrived.
    #[test]
    fn orders_refinements_the_way_the_game_refines_them() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("radiant"), 1, None),
            relic_row("lith_n15_relic", Some("intact"), 1, None),
            relic_row("lith_n15_relic", Some("flawless"), 1, None),
            relic_row("lith_n15_relic", Some("exceptional"), 1, None),
        ];
        let got: Vec<String> = group_refinements(rows)[0]
            .refinements
            .iter()
            .map(|r| r.variant.clone())
            .collect();
        assert_eq!(got, vec!["intact", "exceptional", "flawless", "radiant"]);
    }

    /// A refinement the game adds later must still render rather than vanish.
    #[test]
    fn sorts_an_unrecognised_refinement_last_without_dropping_it() {
        assert!(refinement_order("sublime") > refinement_order("radiant"));
    }

    /// Each refinement keeps its own price: an Intact and a Radiant are
    /// different products and routinely differ by an order of magnitude.
    #[test]
    fn keeps_each_refinements_own_price() {
        let rows = vec![
            relic_row("lith_n15_relic", Some("intact"), 3, Some(4.0)),
            relic_row("lith_n15_relic", Some("radiant"), 1, Some(20.0)),
        ];
        let grouped = group_refinements(rows);
        assert_eq!(grouped[0].refinements[0].price, Some(4.0));
        assert_eq!(grouped[0].refinements[1].price, Some(20.0));
    }
}
