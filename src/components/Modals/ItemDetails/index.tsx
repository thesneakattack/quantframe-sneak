import { Box, Container, Divider } from "@mantine/core";
import { AnalyticsTab, EditTab, OverviewTab, WFMTab } from "./Tabs/index";
import api from "@api/index";
import { useQuery } from "@tanstack/react-query";
import { Loading } from "@components/Shared/Loading";
import { TauriTypes } from "$types";

export enum Operations {
  MarketInfo = "MarketInfo",
  TransactionInfo = "TransactionInfo",
  ProfitabilityInfo = "ProfitabilityInfo",
  EditForm = "EditForm",
}

export type ItemDetailsModalProps = {
  lookup: "stock_item" | "syndicate_item" | "wish_list_item" | "order" | "inventory_item";
  operations: Operations[];
  value: number | string;
  /** Required for the "inventory_item" lookup, which has no record to key on. */
  subType?: TauriTypes.SubType;
  onSave?: (item: TauriTypes.UpdateStockItem | TauriTypes.UpdateSyndicateItem | TauriTypes.UpdateWishListItem) => void;
};
export function ItemDetailsModal({ lookup, operations, value, subType, onSave }: ItemDetailsModalProps) {
  // Don't cache the result of this query
  const { data: dataStockItem } = useQuery({
    queryKey: ["stock_item", value],
    queryFn: () => api.stock_item.getById<{ ui_operations: string[] }>(value as number, operations),
    enabled: lookup === "stock_item",
    gcTime: 0,
  });
  const { data: dataSyndicateItem } = useQuery({
    queryKey: ["syndicate_item", value],
    queryFn: () => api.syndicate_item.getById<{ ui_operations: string[] }>(value as number, operations),
    enabled: lookup === "syndicate_item",
    gcTime: 0,
  });
  const { data: dataWishListItem } = useQuery({
    queryKey: ["wish_list_item", value],
    queryFn: () => api.wish_list.getById<{ ui_operations: string[] }>(value as number, operations),
    enabled: lookup === "wish_list_item",
    gcTime: 0,
  });

  // An inventory row is not a record: it has no id, so it is keyed on what
  // the market uses to identify the product instead.
  const { data: dataInventoryItem } = useQuery({
    queryKey: ["wf_inventory_item_details", value, subType],
    queryFn: () => api.wf_inventory.getItemDetails(value as string, subType),
    enabled: lookup === "inventory_item",
    gcTime: 0,
  });

  const { data: dataOrder } = useQuery({
    queryKey: ["order", value],
    queryFn: () => api.order.getById<{ ui_operations: string[] }>(value as string, operations),
    enabled: lookup === "order",
    gcTime: 0,
  });

  const data =
    lookup === "stock_item"
      ? dataStockItem
      : lookup === "syndicate_item"
        ? dataSyndicateItem
        : lookup === "wish_list_item"
          ? dataWishListItem
          : lookup === "order"
            ? dataOrder
            : dataInventoryItem;

  if (!data)
    return (
      <Box p={"lg"} h={"50vh"}>
        <Loading text="Loading..." />
      </Box>
    );

  return (
    <Container size={"100%"} p={0}>
      <OverviewTab value={data as any} />
      <Divider my="md" hidden={!operations.includes(Operations.MarketInfo)} />
      {operations.includes(Operations.MarketInfo) && <WFMTab value={data as any} />}
      <Divider my="md" hidden={!operations.includes(Operations.TransactionInfo)} />
      {operations.includes(Operations.TransactionInfo) && <AnalyticsTab value={data as any} />}
      <Divider my="md" hidden={!operations.includes(Operations.EditForm)} />
      {operations.includes(Operations.EditForm) && <EditTab lookup={lookup} value={data as any} onSave={onSave} />}
    </Container>
  );
}

