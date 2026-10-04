import { TauriTypes } from "$types";
import { useAppContext } from "@contexts/app.context";
import { ItemName } from "@components/DataDisplay/ItemName";
import { SearchField } from "@components/Forms/SearchField";
import { SortPriority } from "@components/Forms/SortPriority";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faAdd } from "@fortawesome/free-solid-svg-icons";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { useTranslateCommon, useTranslatePages } from "@hooks/useTranslate.hook";
import { Group, NumberFormatter, NumberInput, SegmentedControl, Switch, Text, Tooltip } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { getSafePage, inventoryRowKey } from "@utils/helper";
import { DataTable } from "mantine-datatable";
import { useState } from "react";
import classes from "../../WFInventory.module.css";
import { useModals } from "./modals";
import { useMutations } from "./mutations";
import { useQueries } from "./queries";
import { InventoryInfoAction } from "../../InfoAction";
import { RefreshPriceAction } from "../../RefreshPriceAction";

interface ArcanesPanelProps {
  isActive: boolean;
}

export const ArcanesPanel = ({ isActive }: ArcanesPanelProps) => {
  // Contexts
  const { settings } = useAppContext();
  // States For DataGrid
  const [queryData, setQueryData] = useLocalStorage<TauriTypes.WFItemControllerGetListParams>({
    key: "wf_inventory_arcanes_query_key",
    getInitialValueInEffect: false,
    defaultValue: { page: 1, limit: 25 },
  });
  // States
  const [loadingRows, setLoadingRows] = useState<string[]>([]);
  // The filter panel expands above the table, so the table gives up the room.
  const [filterOpen, setFilterOpen] = useState(false);

  // Translate
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.arcanes.${key}`, { ...context }, i18Key);
  const useTranslateDataGridColumns = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`datatable.columns.${key}`, { ...context }, i18Key);

  // Defaults to the live scraper's own "minimum profit" setting, so the tab
  // hides what the scraper would not bother listing. A negative setting means
  // the scraper has the check disabled, which is no threshold here either.
  const settingMinProfit = settings?.live_scraper.items.wts.min_profit ?? 0;
  const storedMinPrice = (queryData.properties as { min_price?: number } | undefined)?.min_price;
  const minOwned = Number((queryData.properties as { min_owned?: number } | undefined)?.min_owned ?? 0);
  const unvaultedOnly = Boolean((queryData.properties as { unvaulted_only?: boolean } | undefined)?.unvaulted_only);
  const masteredOnly = Boolean((queryData.properties as { mastered_only?: boolean } | undefined)?.mastered_only);
  const minPrice = Number(storedMinPrice ?? Math.max(settingMinProfit, 0));
  const sorts = queryData.sorts ?? [];
  // The stored query may not carry the threshold yet, so send the effective one.
  const effectiveQuery = {
    ...queryData,
    properties: { ...(queryData.properties as object), min_price: minPrice },
  };

  // Queries
  const { arcanesQuery, refetchQueries } = useQueries({ queryData: effectiveQuery, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  const rankFilter = (queryData.properties as { rank_filter?: string } | undefined)?.rank_filter || "all";

  return (
    <>
      <SearchField
        onFilterToggle={setFilterOpen}
        value={queryData.query || ""}
        // Each query rebuilds the projection over the whole inventory, so wait
        // for a pause rather than doing it per character.
        debounce={300}
        onChange={(value) => setQueryData((prev) => ({ ...prev, page: 1, query: value }))}
        filter={
          <Group gap="md" align="flex-end">
            <SortPriority
              columns={[{ value: "name", label: useTranslateCommon("item_name.title") }, { value: "quantity", label: useTranslateDataGridColumns("owned") }, { value: "rank", label: useTranslate("filters.ranked") }, { value: "price", label: useTranslateCommon("datatable_columns.price") }, { value: "max_rank_price", label: useTranslateDataGridColumns("max_rank_price") }]}
              value={sorts}
              onChange={(next) => setQueryData((prev) => ({ ...prev, page: 1, sorts: next }))}
            />
            <SegmentedControl
              value={rankFilter}
              onChange={(value) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), rank_filter: value },
                }))
              }
              data={[
                { label: useTranslate("filters.all"), value: "all" },
                { label: useTranslate("filters.unranked"), value: "unranked" },
                { label: useTranslate("filters.ranked"), value: "ranked" },
              ]}
            />
            <NumberInput
              w={130}
              min={0}
              step={5}
              label={useTranslate("filters.min_price")}
              value={minPrice}
              onChange={(value) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), min_price: Number(value) || 0 },
                }))
              }
            />
            <NumberInput
              w={120}
              min={0}
              step={1}
              label={useTranslate("filters.min_owned")}
              value={minOwned}
              onChange={(value) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), min_owned: Number(value) || 0 },
                }))
              }
            />
            <Switch
              label={useTranslate("filters.unvaulted_only")}
              checked={unvaultedOnly}
              onChange={(event) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), unvaulted_only: event.currentTarget.checked },
                }))
              }
            />
            <Switch
              label={useTranslate("filters.mastered_only")}
              checked={masteredOnly}
              onChange={(event) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), mastered_only: event.currentTarget.checked },
                }))
              }
            />
          </Group>
        }
      />
      <DataTable
        className={`${classes.inventoryTable} ${useHasAlert() ? classes.alert : ""} ${filterOpen ? classes.filterOpen : ""}`}
        mt="md"
        striped
        fetching={arcanesQuery.isLoading}
        records={arcanesQuery.data?.results || []}
        idAccessor={(row) => `${row.unique_name}#${row.sub_type?.rank ?? 0}`}
        page={getSafePage(queryData.page, arcanesQuery.data?.total_pages)}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        totalRecords={arcanesQuery.data?.total || 0}
        recordsPerPage={queryData.limit || 25}
        recordsPerPageOptions={[5, 10, 15, 20, 25, 50, 100]}
        onRecordsPerPageChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
        sortStatus={{
          columnAccessor: sorts[0]?.by || "name",
          direction: sorts[0]?.direction || "asc",
        }}
        onSortStatusChange={(sort) => {
          if (!sort || !sort.columnAccessor) return;
          // A header click is the quick path: it replaces the priority list
          // with that one column. The filter panel edits the full list.
          setQueryData((prev) => ({
            ...prev,
            page: 1,
            sorts: [{ by: sort.columnAccessor as string, direction: sort.direction }],
          }));
        }}
        columns={[
          {
            accessor: "name",
            title: useTranslateCommon("item_name.title"),
            sortable: true,
            render: (row) => <ItemName color="gray.4" size="md" value={row} hideQuantity />,
          },
          {
            accessor: "quantity",
            title: useTranslateDataGridColumns("owned"),
            sortable: true,
            width: 100,
          },
          {
            accessor: "price",
            title: useTranslateCommon("datatable_columns.price"),
            sortable: true,
            width: 110,
            render: (row) => {
              const price = row.properties?.price;
              // Never fetched on demand: the figure is whatever the shipped
              // statistics and the live scraper have already recorded.
              if (price == null)
                return (
                  <Tooltip label={useTranslateCommon("datatable_columns.price_unknown")}>
                    <Text c="dimmed">?</Text>
                  </Tooltip>
                );
              return (
                <Group gap={4}>
                  <NumberFormatter value={Math.round(price)} thousandSeparator="." decimalSeparator="," />
                  <Text c="dimmed" size="xs">
                    p
                  </Text>
                </Group>
              );
            },
          },
          {
            accessor: "max_rank_price",
            title: useTranslateDataGridColumns("max_rank_price"),
            sortable: true,
            width: 110,
            render: (row) => {
              const price = row.properties?.max_rank_price;
              const maxRank = row.properties?.max_rank;
              // An item with no ranks has no separate maxed product, so there
              // is nothing to show rather than an unknown to chase.
              if (maxRank == null || maxRank <= 0) return <Text c="dimmed">-</Text>;
              if (price == null)
                return (
                  <Tooltip label={useTranslateCommon("datatable_columns.price_unknown")}>
                    <Text c="dimmed">?</Text>
                  </Tooltip>
                );
              return (
                <Group gap={4}>
                  <NumberFormatter value={Math.round(price)} thousandSeparator="." decimalSeparator="," />
                  <Text c="dimmed" size="xs">
                    p
                  </Text>
                </Group>
              );
            },
          },
          {
            accessor: "actions",
            title: useTranslateCommon("datatable_columns.actions.title"),
            width: 140,
            render: (row) => (
              <Group gap={2} wrap="nowrap">
              <InventoryInfoAction wfmUrl={row.wfm_url} subType={row.sub_type} />
              <RefreshPriceAction wfmUrl={row.wfm_url} subType={row.sub_type} maxRank={row.properties?.max_rank} onRefreshed={refetchQueries} />
              <ActionWithTooltip
                icon={faAdd}
                color={row.properties?.is_in_stock ? "var(--mantine-color-green-6)" : "var(--mantine-color-red-6)"}
                actionProps={{ size: "sm", loading: loadingRows.includes(inventoryRowKey(row.wfm_url, row.sub_type)) }}
                iconProps={{ size: "xs" }}
                tooltip={useTranslate(`stock_status.${row.properties?.is_in_stock ? "found" : "not_found"}`)}
                onClick={() => OpenAddToStockModal(row)}
              />
              </Group>
            ),
          },
        ]}
      />
    </>
  );
};
