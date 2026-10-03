import { TauriTypes } from "$types";
import { ItemName } from "@components/DataDisplay/ItemName";
import { SearchField } from "@components/Forms/SearchField";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faAdd } from "@fortawesome/free-solid-svg-icons";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { useTranslateCommon, useTranslatePages } from "@hooks/useTranslate.hook";
import { Group, NumberFormatter, SegmentedControl, Text } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { getSafePage } from "@utils/helper";
import { DataTable } from "mantine-datatable";
import { useState } from "react";
import classes from "../../WFInventory.module.css";
import { useModals } from "./modals";
import { useMutations } from "./mutations";
import { useQueries } from "./queries";

interface ModsPanelProps {
  isActive: boolean;
}

export const ModsPanel = ({ isActive }: ModsPanelProps) => {
  // States For DataGrid
  const [queryData, setQueryData] = useLocalStorage<TauriTypes.WFItemControllerGetListParams>({
    key: "wf_inventory_mods_query_key",
    getInitialValueInEffect: false,
    defaultValue: { page: 1, limit: 25 },
  });
  // States
  const [loadingRows, setLoadingRows] = useState<string[]>([]);

  // Translate
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.tabs.mods.${key}`, { ...context }, i18Key);
  const useTranslateDataGridColumns = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`datatable.columns.${key}`, { ...context }, i18Key);

  // Queries
  const { modsQuery, refetchQueries } = useQueries({ queryData, isActive });
  const { createMutation } = useMutations({ refetchQueries, setLoadingRows });
  const { OpenAddToStockModal } = useModals({ createMutation });

  const rankFilter = (queryData.properties as { rank_filter?: string } | undefined)?.rank_filter || "all";

  return (
    <>
      <SearchField
        value={queryData.query || ""}
        onChange={(value) => setQueryData((prev) => ({ ...prev, page: 1, query: value }))}
        filter={
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
        }
      />
      <DataTable
        className={`${classes.container} ${useHasAlert() ? classes.alert : ""}`}
        mt="md"
        striped
        fetching={modsQuery.isLoading}
        records={modsQuery.data?.results || []}
        idAccessor={(row) => `${row.unique_name}#${row.sub_type?.rank ?? 0}`}
        page={getSafePage(queryData.page, modsQuery.data?.total_pages)}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        totalRecords={modsQuery.data?.total || 0}
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
            render: (row) =>
              row.properties?.price != null ? (
                <Group gap={4}>
                  <NumberFormatter value={Math.round(row.properties.price)} thousandSeparator="." decimalSeparator="," />
                  <Text c="dimmed" size="xs">
                    p
                  </Text>
                </Group>
              ) : (
                <Text c="dimmed">—</Text>
              ),
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
