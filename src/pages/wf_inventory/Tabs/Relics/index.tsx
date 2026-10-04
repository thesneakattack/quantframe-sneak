import { TauriTypes } from "$types";
import { ItemName } from "@components/DataDisplay/ItemName";
import { SearchField } from "@components/Forms/SearchField";
import { SortPriority } from "@components/Forms/SortPriority";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faAdd } from "@fortawesome/free-solid-svg-icons";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { useTranslateCommon, useTranslatePages } from "@hooks/useTranslate.hook";
import { Badge, Group, NumberFormatter, NumberInput, Stack, Text, Tooltip } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { getSafePage, inventoryRowKey } from "@utils/helper";
import { DataTable } from "mantine-datatable";
import { useState } from "react";
import classes from "../../WFInventory.module.css";
import { useModals } from "./modals";
import { useMutations } from "./mutations";
import { useQueries } from "./queries";
import { InventoryInfoAction } from "../../InfoAction";

interface RelicsPanelProps {
  isActive: boolean;
}

const Price = ({ value, unknownLabel }: { value?: number | null; unknownLabel: string }) => {
  if (value == null)
    return (
      <Tooltip label={unknownLabel}>
        <Text c="dimmed">?</Text>
      </Tooltip>
    );
  return (
    <Group gap={4}>
      <NumberFormatter value={Math.round(value)} thousandSeparator="." decimalSeparator="," />
      <Text c="dimmed" size="xs">
        p
      </Text>
    </Group>
  );
};

export const RelicsPanel = ({ isActive }: RelicsPanelProps) => {
  const [queryData, setQueryData] = useLocalStorage<TauriTypes.WFItemControllerGetListParams>({
    key: "wf_inventory_relics_query_key",
    getInitialValueInEffect: false,
    defaultValue: { page: 1, limit: 25 },
  });
  const [loadingRows, setLoadingRows] = useState<string[]>([]);
  const [filterOpen, setFilterOpen] = useState(false);

  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.relics.${key}`, { ...context }, i18Key);
  const useTranslateDataGridColumns = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`datatable.columns.${key}`, { ...context }, i18Key);

  const minOwned = Number((queryData.properties as { min_owned?: number } | undefined)?.min_owned ?? 0);
  const sorts = queryData.sorts ?? [];
  const priceUnknown = useTranslateCommon("datatable_columns.price_unknown");

  const { relicsQuery, refetchQueries } = useQueries({ queryData, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  return (
    <>
      <SearchField
        onFilterToggle={setFilterOpen}
        value={queryData.query || ""}
        debounce={300}
        onChange={(value) => setQueryData((prev) => ({ ...prev, page: 1, query: value }))}
        filter={
          <Group gap="md" align="flex-end">
            <SortPriority
              columns={[
                { value: "name", label: useTranslateCommon("item_name.title") },
                { value: "total_owned", label: useTranslateDataGridColumns("owned") },
                { value: "price", label: useTranslateCommon("datatable_columns.price") },
              ]}
              value={sorts}
              onChange={(value) => setQueryData((prev) => ({ ...prev, page: 1, sorts: value }))}
            />
            <NumberInput
              label={useTranslate("filters.min_owned")}
              value={minOwned}
              min={0}
              w={120}
              onChange={(value) =>
                setQueryData((prev) => ({
                  ...prev,
                  page: 1,
                  properties: { ...(prev.properties as object), min_owned: Number(value) || 0 },
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
        fetching={relicsQuery.isLoading}
        records={relicsQuery.data?.results || []}
        idAccessor={(row) => row.wfm_url}
        page={getSafePage(queryData.page, relicsQuery.data?.total_pages)}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        totalRecords={relicsQuery.data?.total || 0}
        recordsPerPage={queryData.limit || 25}
        recordsPerPageOptions={[5, 10, 15, 20, 25, 50, 100]}
        onRecordsPerPageChange={(limit) => setQueryData((prev) => ({ ...prev, page: 1, limit }))}
        sortStatus={{
          columnAccessor: sorts[0]?.by || "price",
          direction: sorts[0]?.direction || "desc",
        }}
        onSortStatusChange={(sort) => {
          if (!sort || !sort.columnAccessor) return;
          setQueryData((prev) => ({
            ...prev,
            page: 1,
            sorts: [{ by: sort.columnAccessor as string, direction: sort.direction }],
          }));
        }}
        rowExpansion={{
          content: ({ record }) => (
            <Stack gap={4} p="xs">
              {record.refinements.map((tier) => (
                <Group key={tier.variant} justify="space-between" px="md">
                  <Badge variant="light" w={110}>
                    {tier.variant}
                  </Badge>
                  <Text size="sm" w={80}>
                    {tier.owned}
                  </Text>
                  <Price value={tier.price} unknownLabel={priceUnknown} />
                  <InventoryInfoAction wfmUrl={record.wfm_url} subType={{ variant: tier.variant }} />
                  <ActionWithTooltip
                    icon={faAdd}
                    color="var(--mantine-color-blue-6)"
                    actionProps={{
                      size: "sm",
                      loading: loadingRows.includes(inventoryRowKey(record.wfm_url, { variant: tier.variant })),
                    }}
                    iconProps={{ size: "xs" }}
                    tooltip={useTranslate("add_refinement")}
                    onClick={() => OpenAddToStockModal(record, tier)}
                  />
                </Group>
              ))}
            </Stack>
          ),
        }}
        columns={[
          {
            accessor: "name",
            title: useTranslateCommon("item_name.title"),
            sortable: true,
            render: (row) => <ItemName color="gray.4" size="md" value={row} hideQuantity />,
          },
          {
            accessor: "refinements",
            title: useTranslateDataGridColumns("refinements"),
            width: 170,
            render: (row) => (
              <Group gap={4}>
                {row.refinements.map((tier) => (
                  <Tooltip key={tier.variant} label={`${tier.variant}: ${tier.owned}`}>
                    <Badge size="sm" variant="light">
                      {tier.variant.slice(0, 3)} {tier.owned}
                    </Badge>
                  </Tooltip>
                ))}
              </Group>
            ),
          },
          {
            accessor: "total_owned",
            title: useTranslateDataGridColumns("owned"),
            sortable: true,
            width: 100,
          },
          {
            accessor: "price",
            title: useTranslateDataGridColumns("best_price"),
            sortable: true,
            width: 110,
            // The best tier's price: that is what the relic is worth to
            // sell, and what the default sort orders by.
            render: (row) => <Price value={row.properties?.price} unknownLabel={priceUnknown} />,
          },
        ]}
      />
    </>
  );
};
