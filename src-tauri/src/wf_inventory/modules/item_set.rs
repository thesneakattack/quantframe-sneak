use std::sync::{Arc, Weak};

use entity::{dto::PaginatedResult, enums::FieldChange};
use utils::Error;

use crate::{
    cache::modules::{build_price_index, lookup_price},
    helper::paginate,
    utils::modules::states,
    wf_inventory::{
        item_base::WFInvItemBase,
        modules::item::{meets_min_price, owned_counts},
        *,
    },
};

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
    pub fn get_sets(&self, query: WFItemPaginationDto) -> Result<PaginatedResult<WFInvSet>, Error> {
        let client = self.client.upgrade().unwrap();
        let root = client.get_root();
        let cache = states::cache_client()?;

        let counts = owned_counts(&[&root.recipes, &root.misc_items]);
        let prices = build_price_index(&cache.item_price().get_items()?);

        let mut sets: Vec<WFInvSet> = Vec::new();
        for cache_set in cache.item_set().get_all_sets()? {
            let members: Vec<WFInvSetMember> = cache_set
                .members
                .iter()
                .map(|member| WFInvSetMember {
                    unique_name: member.unique_name.clone(),
                    name: member.name.clone(),
                    have: counts.get(&member.unique_name).copied().unwrap_or(0),
                    required: member.required,
                    is_main_blueprint: member.is_main_blueprint,
                })
                .collect();

            // Drop sets the player holds no piece of at all; the rest are
            // shown so the user can see what to buy next.
            if members.iter().all(|m| m.have == 0) {
                continue;
            }
            let owned_members = satisfied_members(&members);

            let copies = complete_copies(&members);
            let mut base = WFInvItemBase {
                id: cache_set.set.wfm_id.clone(),
                name: cache_set.set.name.clone(),
                unique_name: cache_set.set.unique_name.clone(),
                wfm_url: cache_set.set.wfm_url.clone(),
                quantity: copies,
                sub_type: None,
                ..Default::default()
            };
            base.properties
                .set_property_value("tags", cache_set.set.tags.clone());
            base.properties.set_property_value(
                "price",
                lookup_price(&prices, &cache_set.set.wfm_url, None).or_else(|| {
                    // Filled in by an earlier page view; cache-only, never
                    // an HTTP call from a table query.
                    crate::market_prices::MarketPriceStore::get().remembered_price(
                        &crate::market_prices::PriceKey {
                            wfm_url: cache_set.set.wfm_url.clone(),
                            rank: None,
                            variant: None,
                        },
                    )
                }),
            );

            sets.push(WFInvSet {
                base,
                total_members: members.len() as i64,
                owned_members,
                complete_copies: copies,
                members,
            });
        }

        if let FieldChange::Value(text) = &query.query {
            let text = text.to_lowercase();
            sets.retain(|set| set.base.name.to_lowercase().contains(&text));
        }
        let complete_only = match &query.properties {
            FieldChange::Value(properties) => properties.get_property_value("complete_only", false),
            _ => false,
        };
        if complete_only {
            sets.retain(|set| set.complete_copies > 0);
        }

        let min_price = match &query.properties {
            FieldChange::Value(properties) => properties.get_property_value("min_price", 0.0f64),
            _ => 0.0,
        };
        sets.retain(|set| {
            meets_min_price(
                set.base
                    .properties
                    .get_property_value::<Option<f64>>("price", None),
                Some(min_price),
            )
        });

        // Complete sets first by copies descending, then partials by fewest
        // members missing, so the nearly-complete ones are what you see.
        sets.sort_by(|a, b| {
            b.complete_copies
                .cmp(&a.complete_copies)
                .then((a.total_members - a.owned_members).cmp(&(b.total_members - b.owned_members)))
                .then(a.base.name.cmp(&b.base.name))
        });

        Ok(paginate(
            &sets,
            query.pagination.page,
            query.pagination.limit,
        ))
    }
}
