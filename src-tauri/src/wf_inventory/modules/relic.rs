use std::sync::{Arc, Weak};

use entity::dto::PaginatedResult;
use utils::Error;

use crate::{
    helper::paginate,
    wf_inventory::{
        item_base::WFInvItemBase,
        modules::item::{apply_common_filters, priced_rows},
        *,
    },
};

/// A Relics row's value for a sortable column.
fn relic_value(relic: &WFInvRelic, column: &str) -> SortValue {
    match column {
        "total_owned" => SortValue::Num(relic.total_owned as f64),
        "refinements" => SortValue::Num(relic.refinements.len() as f64),
        "price" => SortValue::MaybeNum(best_price(relic)),
        _ => SortValue::Text(relic.base.name.clone()),
    }
}

#[derive(Debug)]
pub struct RelicsModule {
    client: Weak<WFInventoryState>,
}

impl RelicsModule {
    pub fn new(client: Arc<WFInventoryState>) -> Arc<Self> {
        Arc::new(Self {
            client: Arc::downgrade(&client),
        })
    }

    /// Every relic the player holds, one row per relic with its refinements
    /// grouped underneath.
    ///
    /// Grouping happens here rather than in the snapshot because a refinement
    /// carries its own price, and prices are deliberately not baked into the
    /// snapshot - they arrive from the market as the backfill resolves them
    /// and must show without rebuilding every row.
    pub fn get_relics(
        &self,
        query: WFItemPaginationDto,
    ) -> Result<PaginatedResult<WFInvRelic>, Error> {
        let rows = self.client.upgrade().unwrap().rows()?;
        let mut relics = group_refinements(priced_rows(&rows.relics)?);

        // The grouped row carries its best refinement's price, so the tab can
        // render and the common filters can judge a relic by what it is
        // actually worth rather than by a refinement nobody would sell.
        for relic in relics.iter_mut() {
            let price = best_price(relic);
            relic.base.properties.set_property_value("price", price);
        }

        // The common filters read a flat row, so they are applied to the
        // representative base and the survivors select the grouped rows. The
        // base carries the best refinement's price so a minimum-price filter
        // judges the relic by what it is actually worth.
        let mut bases: Vec<WFInvItemBase> = relics.iter().map(|r| r.base.clone()).collect();
        apply_common_filters(&mut bases, &query);
        let kept: std::collections::HashSet<String> =
            bases.into_iter().map(|base| base.wfm_url).collect();
        relics.retain(|relic| kept.contains(&relic.base.wfm_url));

        let sorts = query.sort_fields();
        if sorts.is_empty() {
            // The default view answers "what is worth listing": dearest first,
            // and an unpriced relic is not evidence of being cheap, so it
            // trails rather than leading.
            relics.sort_by(|a, b| match (best_price(b), best_price(a)) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => a.base.name.cmp(&b.base.name),
            });
        } else {
            sort_by_fields(&mut relics, &sorts, relic_value);
        }

        Ok(paginate(
            &relics,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}
