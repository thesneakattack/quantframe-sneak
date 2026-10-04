use std::sync::{Arc, Weak};

use entity::dto::PaginatedResult;
use utils::Error;

use crate::{
    helper::paginate,
    wf_inventory::{
        item_base::WFInvItemBase,
        modules::item::{apply_common_filters, flag, priced_rows},
        *,
    },
};

/// A Sets row's value for a sortable column.
fn set_value(set: &WFInvSet, column: &str) -> SortValue {
    match column {
        "complete_copies" => SortValue::Num(set.complete_copies as f64),
        "owned_members" => SortValue::Num(set.owned_members as f64),
        "missing" => SortValue::Num((set.total_members - set.owned_members) as f64),
        "price" => SortValue::MaybeNum(set.base.properties.get_property_value("price", None)),
        _ => SortValue::Text(set.base.name.clone()),
    }
}

#[derive(Debug)]
pub struct SetsModule {
    client: Weak<WFInventoryState>,
}

impl SetsModule {
    pub fn new(client: Arc<WFInventoryState>) -> Arc<Self> {
        Arc::new(Self {
            client: Arc::downgrade(&client),
        })
    }

    /// Every set the player holds at least one member of, complete ones first
    /// and the rest ordered by how few members are missing - the partials are
    /// there to show what to buy next.
    /// Every set the player holds a piece of, complete ones first.
    pub fn get_sets(&self, query: WFItemPaginationDto) -> Result<PaginatedResult<WFInvSet>, Error> {
        let rows = self.client.upgrade().unwrap().rows()?;

        // Sets carry their price on the embedded row, so they are priced
        // through the same path as the other tabs.
        let bases: Vec<WFInvItemBase> = rows.sets.iter().map(|s| s.base.clone()).collect();
        let priced = priced_rows(&bases)?;
        let mut sets: Vec<WFInvSet> = rows
            .sets
            .iter()
            .zip(priced)
            .map(|(set, base)| WFInvSet {
                base,
                members: set.members.clone(),
                complete_copies: set.complete_copies,
                owned_members: set.owned_members,
                total_members: set.total_members,
            })
            .collect();

        if flag(&query, "complete_only") {
            sets.retain(|set| set.complete_copies > 0);
        }
        let mut bases: Vec<WFInvItemBase> = sets.iter().map(|s| s.base.clone()).collect();
        apply_common_filters(&mut bases, &query);
        let kept: std::collections::HashSet<String> =
            bases.into_iter().map(|b| b.unique_name).collect();
        sets.retain(|set| kept.contains(&set.base.unique_name));

        let sorts = query.sort_fields();
        if sorts.is_empty() {
            // The default view: complete sets first by copies descending, then
            // partials by fewest members missing, so the nearly-complete ones
            // are what you see.
            sets.sort_by(|a, b| {
                b.complete_copies
                    .cmp(&a.complete_copies)
                    .then(
                        (a.total_members - a.owned_members)
                            .cmp(&(b.total_members - b.owned_members)),
                    )
                    .then(a.base.name.cmp(&b.base.name))
            });
        } else {
            sort_by_fields(&mut sets, &sorts, set_value);
        }

        Ok(paginate(
            &sets,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}
