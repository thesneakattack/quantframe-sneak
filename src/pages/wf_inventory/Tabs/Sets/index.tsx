import { TauriTypes } from "$types";
import { useAppContext } from "@contexts/app.context";
import { ItemName } from "@components/DataDisplay/ItemName";
import { SearchField } from "@components/Forms/SearchField";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faAdd } from "@fortawesome/free-solid-svg-icons";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { useTranslateCommon, useTranslatePages } from "@hooks/useTranslate.hook";
import { Badge, Box, Group, NumberFormatter, NumberInput, Stack, Switch, Text, Tooltip } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { getSafePage } from "@utils/helper";
import { DataTable } from "mantine-datatable";
import { useState } from "react";
import classes from "../../WFInventory.module.css";
import { useModals } from "./modals";
import { useMutations } from "./mutations";
import { useQueries } from "./queries";

interface SetsPanelProps {
  isActive: boolean;
}

export const SetsPanel = ({ isActive }: SetsPanelProps) => {
  // Contexts
  const { settings } = useAppContext();
  // States For DataGrid
  const [queryData, setQueryData] = useLocalStorage<TauriTypes.WFItemControllerGetListParams>({
    key: "wf_inventory_sets_query_key",
    getInitialValueInEffect: false,
    defaultValue: { page: 1, limit: 25 },
  });
  // States
  const [loadingRows, setLoadingRows] = useState<string[]>([]);
  // The filter panel expands above the table, so the table gives up the room.
  const [filterOpen, setFilterOpen] = useState(false);

  // Translate
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.sets.${key}`, { ...context }, i18Key);
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
  const { setsQuery, refetchQueries } = useQueries({ queryData: effectiveQuery, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  const completeOnly = Boolean((queryData.properties as { complete_only?: boolean } | undefined)?.complete_only);

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
              label={useTranslate("filters.complete_only")}
              checked={completeOnly}
              onChange={(event) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), complete_only: event.currentTarget.checked },
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
        fetching={setsQuery.isLoading}
        records={setsQuery.data?.results || []}
        idAccessor="unique_name"
        page={getSafePage(queryData.page, setsQuery.data?.total_pages)}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        totalRecords={setsQuery.data?.total || 0}
        recordsPerPage={queryData.limit || 25}
        recordsPerPageOptions={[5, 10, 15, 20, 25, 50, 100]}
        onRecordsPerPageChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
        rowExpansion={{
          content: ({ record }) => (
            <Stack gap={2} p="sm">
              {record.members.map((member) => (
                <Group key={member.unique_name} gap="xs" justify="space-between" px="md">
                  <Text size="sm">{member.name}</Text>
                  <Text size="sm" c={member.have >= member.required ? "green.6" : "red.6"}>
                    {member.have} / {member.required}
                  </Text>
                </Group>
              ))}
            </Stack>
          ),
        }}
        columns={[
          {
            accessor: "name",
            title: useTranslateCommon("item_name.title"),
            render: (row) => <ItemName color="gray.4" size="md" value={row} hideQuantity />,
          },
          {
            accessor: "owned_members",
            title: useTranslateDataGridColumns("owned_members"),
            width: 130,
            render: (row) => (
              <Text c={row.owned_members === row.total_members ? undefined : "dimmed"}>
                {row.owned_members} / {row.total_members}
              </Text>
            ),
          },
          {
            accessor: "complete_copies",
            title: useTranslateDataGridColumns("complete"),
            width: 150,
            render: (row) =>
              row.complete_copies > 0 ? (
                <Badge color="green.7">{useTranslate("copies", { count: row.complete_copies })}</Badge>
              ) : (
                <Badge color="gray.7" variant="light">
                  {useTranslate("missing", { count: row.total_members - row.owned_members })}
                </Badge>
              ),
          },
          {
            accessor: "price",
            title: useTranslateCommon("datatable_columns.price"),
            sortable: true,
            width: 110,
            render: (row) =>
              row.properties?.price != null ? (
                <Group gap={4}>
                  <NumberFormatter value={Math.round(row.properties.price)} thousandSeparator="." decimalSeparator="," />
                  <Text c="dimmed" size="xs">
                    p
                  </Text>
                </Group>
              ) : (
                // Unknown, not cheap: these are never hidden by the minimum
                // price filter, because a good many of them turn out to be
                // worth more than the threshold.
                <Tooltip label={useTranslateCommon("datatable_columns.price_unknown")}>
                  <Text c="dimmed">?</Text>
                </Tooltip>
              ),
          },
          {
            accessor: "actions",
            title: useTranslateCommon("datatable_columns.actions.title"),
            width: 80,
            render: (row) => (
              // Only complete sets can be listed; partials are here to show
              // which component to hunt for next.
              <Box>
                {row.complete_copies > 0 && (
                  <ActionWithTooltip
                    icon={faAdd}
                    color={row.properties?.is_in_stock ? "var(--mantine-color-green-6)" : "var(--mantine-color-red-6)"}
                    actionProps={{ size: "sm", loading: loadingRows.includes(row.wfm_url) }}
                    iconProps={{ size: "xs" }}
                    tooltip={useTranslate(`stock_status.${row.properties?.is_in_stock ? "found" : "not_found"}`)}
                    onClick={() => OpenAddToStockModal(row)}
                  />
                )}
              </Box>
            ),
          },
        ]}
      />
    </>
  );
};
