import { TauriTypes } from "$types";
import { useAppContext } from "@contexts/app.context";
import { ItemName } from "@components/DataDisplay/ItemName";
import { SearchField } from "@components/Forms/SearchField";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faAdd, faTriangleExclamation } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { useResolvedPrices } from "@hooks/useResolvedPrices.hook";
import { useTranslateCommon, useTranslatePages } from "@hooks/useTranslate.hook";
import { Group, Loader, NumberFormatter, NumberInput, Switch, Text, Tooltip } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { getSafePage } from "@utils/helper";
import { DataTable } from "mantine-datatable";
import { useState } from "react";
import classes from "../../WFInventory.module.css";
import { useModals } from "./modals";
import { useMutations } from "./mutations";
import { useQueries } from "./queries";

interface PartsPanelProps {
  isActive: boolean;
}

export const PartsPanel = ({ isActive }: PartsPanelProps) => {
  // Contexts
  const { settings } = useAppContext();
  // States For DataGrid
  const [queryData, setQueryData] = useLocalStorage<TauriTypes.WFItemControllerGetListParams>({
    key: "wf_inventory_parts_query_key",
    getInitialValueInEffect: false,
    defaultValue: { page: 1, limit: 25 },
  });
  // States
  const [loadingRows, setLoadingRows] = useState<string[]>([]);
  // The filter panel expands above the table, so the table gives up the room.
  const [filterOpen, setFilterOpen] = useState(false);

  // Translate
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.parts.${key}`, { ...context }, i18Key);
  const useTranslateDataGridColumns = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`datatable.columns.${key}`, { ...context }, i18Key);

  // Defaults to the live scraper's own "minimum profit" setting, so the tab
  // hides what the scraper would not bother listing. A negative setting means
  // the scraper has the check disabled, which is no threshold here either.
  const settingMinProfit = settings?.live_scraper.items.wts.min_profit ?? 0;
  const storedMinPrice = (queryData.properties as { min_price?: number } | undefined)?.min_price;
  const minPrice = Number(storedMinPrice ?? Math.max(settingMinProfit, 0));
  // The stored query may not carry the threshold yet, so send the effective one.
  const effectiveQuery = {
    ...queryData,
    properties: { ...(queryData.properties as object), min_price: minPrice },
  };

  // Queries
  const { partsQuery, refetchQueries } = useQueries({ queryData: effectiveQuery, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { resolvedPrice } = useResolvedPrices(partsQuery.data?.results);
  const { OpenAddToStockModal } = useModals({ createMutation });

  const inSetOnly = Boolean((queryData.properties as { in_set_only?: boolean } | undefined)?.in_set_only);

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
            <Switch
              label={useTranslate("filters.in_set_only")}
              checked={inSetOnly}
              onChange={(event) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), in_set_only: event.currentTarget.checked },
                }))
              }
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
          </Group>
        }
      />
      <DataTable
        className={`${classes.inventoryTable} ${useHasAlert() ? classes.alert : ""} ${filterOpen ? classes.filterOpen : ""}`}
        mt="md"
        striped
        fetching={partsQuery.isLoading}
        records={partsQuery.data?.results || []}
        idAccessor="unique_name"
        page={getSafePage(queryData.page, partsQuery.data?.total_pages)}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        totalRecords={partsQuery.data?.total || 0}
        recordsPerPage={queryData.limit || 25}
        recordsPerPageOptions={[5, 10, 15, 20, 25, 50, 100]}
        onRecordsPerPageChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
        sortStatus={{
          columnAccessor: queryData.sort_by || "name",
          direction: queryData.sort_direction || "asc",
        }}
        onSortStatusChange={(sort) => {
          if (!sort || !sort.columnAccessor) return;
          setQueryData((prev) => ({ ...prev, sort_by: sort.columnAccessor as string, sort_direction: sort.direction }));
        }}
        columns={[
          {
            accessor: "name",
            title: useTranslateCommon("item_name.title"),
            sortable: true,
            render: (row) => (
              <Group gap={6}>
                <ItemName color="gray.4" size="md" value={row} hideQuantity />
                {(row.properties?.in_stock_sets?.length || 0) > 0 && (
                  <Tooltip label={useTranslate("stock_set_conflict", { sets: (row.properties?.in_stock_sets || []).join(", ") })}>
                    <Text c="yellow.7" component="span">
                      <FontAwesomeIcon icon={faTriangleExclamation} />
                    </Text>
                  </Tooltip>
                )}
              </Group>
            ),
          },
          {
            accessor: "quantity",
            title: useTranslateDataGridColumns("owned"),
            sortable: true,
            width: 100,
          },
          {
            accessor: "in_sets",
            title: useTranslateDataGridColumns("set"),
            render: (row) => <Text c="dimmed">{(row.properties?.in_sets || []).join(", ")}</Text>,
          },
          {
            accessor: "price",
            title: useTranslateCommon("datatable_columns.price"),
            sortable: true,
            width: 110,
            render: (row) => {
              const price = resolvedPrice(row);
              // undefined means still being looked up, null means asked and
              // nothing traded. Neither is a reason to read the row as cheap.
              if (price === undefined) return <Loader size="xs" color="gray.6" />;
              if (price === null)
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
            width: 80,
            render: (row) => (
              <ActionWithTooltip
                icon={faAdd}
                color={row.properties?.is_in_stock ? "var(--mantine-color-green-6)" : "var(--mantine-color-red-6)"}
                actionProps={{ size: "sm", loading: loadingRows.includes(row.wfm_url) }}
                iconProps={{ size: "xs" }}
                tooltip={useTranslate(`stock_status.${row.properties?.is_in_stock ? "found" : "not_found"}`)}
                onClick={() => OpenAddToStockModal(row)}
              />
            ),
          },
        ]}
      />
    </>
  );
};
