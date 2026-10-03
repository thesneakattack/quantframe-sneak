import { TauriTypes } from "$types";
import api from "@api/index";
import { useQuery } from "@tanstack/react-query";

/**
 * Fills in prices the bundled cache does not carry, for the rows on screen.
 *
 * The shipped price data covers only part of the inventory, so a row can come
 * back with no price at all. Those rows are asked about individually, but only
 * the ones actually visible, so the cost follows what you look at rather than
 * how large the inventory is. Answers are remembered on the Rust side,
 * including "nothing traded", so paging back is free.
 */
export const useResolvedPrices = (rows: TauriTypes.WFInvItemRow[] | undefined) => {
  const missing = (rows || [])
    .filter((row) => row.properties?.price == null)
    .map((row) => ({
      wfm_url: row.wfm_url,
      rank: row.sub_type?.rank ?? null,
      variant: row.sub_type?.variant ?? null,
    }));

  // Keyed by the exact set of rows asked about, so switching page or filter
  // starts a new lookup and returning to a page reuses the previous answer.
  const key = missing.map((m) => `${m.wfm_url}#${m.rank ?? 0}#${m.variant ?? ""}`);

  const query = useQuery({
    queryKey: ["wf_inventory_resolve_prices", key],
    queryFn: () => api.wf_inventory.resolvePrices(missing),
    enabled: missing.length > 0,
    retry: false,
    staleTime: 60 * 60 * 1000,
  });

  /** The resolved price for a row, or undefined while it is still unknown. */
  const resolvedPrice = (row: TauriTypes.WFInvItemRow): number | null | undefined => {
    if (row.properties?.price != null) return row.properties.price;
    if (!query.data) return undefined;
    return query.data[`${row.wfm_url}#${row.sub_type?.rank ?? 0}#${row.sub_type?.variant ?? ""}`] ?? null;
  };

  return { resolvedPrice, isResolving: query.isFetching };
};
