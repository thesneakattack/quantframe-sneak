import { TauriTypes } from "$types";
import api, { HasPermission } from "@api/index";
import { ItemName } from "@components/DataDisplay/ItemName";
import { SearchField } from "@components/Forms/SearchField";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { ColorInfo } from "@components/Shared/ColorInfo";
import { StatsWithSegments } from "@components/Shared/StatsWithSegments";
import { useLiveScraperContext } from "@contexts/liveScraper.context";
import { faDownload, faEdit, faPlay, faTrashCan } from "@fortawesome/free-solid-svg-icons";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { useTauriEvent } from "@hooks/useTauriEvent.hook";
import { useTranslateCommon, useTranslateEnums, useTranslatePages } from "@hooks/useTranslate.hook";
import { Box, Grid, Group, NumberFormatter } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { notifications } from "@mantine/notifications";
import { useQuery } from "@tanstack/react-query";
import { getSafePage } from "@utils/helper";
import { DataTable } from "mantine-datatable";
import { useEffect, useState } from "react";
import { ColumnActions } from "../../Columns/ColumnActions";
import { ColumnMinMaxPrice } from "../../Columns/ColumnMinMaxPrice";
import classes from "../../LiveScraper.module.css";
import { useModals } from "./modals";
import { useMutations } from "./mutations";
import { useSyndicateItemQueries } from "./queries";

interface SyndicatePanelProps {
  isActive?: boolean;
}

export const SyndicatePanel = ({ isActive }: SyndicatePanelProps = {}) => {
  // Contexts
  const { is_running } = useLiveScraperContext();
  // States For DataGrid
  const [queryData, setQueryData] = useLocalStorage<TauriTypes.SyndicateItemControllerGetListParams>({
    key: "syndicate_item_query_key",
    getInitialValueInEffect: false,
    defaultValue: { page: 1, limit: 10 },
  });
  // States
  const [loadingRows, setLoadingRows] = useState<string[]>([]);
  const [canExport, setCanExport] = useState<boolean>(false);
  const [selectedRecords, setSelectedRecords] = useState<TauriTypes.SyndicateItem[]>([]);

  // Check permissions for export on mount
  useEffect(() => {
    HasPermission(TauriTypes.PermissionsFlags.EXPORT_DATA).then((res) => setCanExport(res));
  }, []);

  // Translate
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`live_scraper.${key}`, { ...context }, i18Key);
  const useTranslateTabItem = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`tabs.syndicate.${key}`, { ...context }, i18Key);
  const useTranslateSegments = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`segments.${key}`, { ...context }, i18Key);
  const useTranslateStockStatus = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateEnums(`stock_status.${key}`, { ...context }, i18Key);
  const useTranslateDataGridColumns = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateTabItem(`datatable.columns.${key}`, { ...context }, i18Key);

  // Fetch data from rust side
  const { data } = useQuery({
    queryKey: ["cache_syndicates"],
    queryFn: () => api.cache.getSyndicates(),
  });

  const GetSyndicateColor = (syndicateName: string) => {
    if (!data) return "gray.4";
    const syndicate = data.find((s) => s.name === syndicateName);
    return syndicate?.backgroundColour || "gray";
  };

  // Queries
  const { paginationQuery, financialReportQuery, statusCountsQuery, syndicateCountsQuery, refetchQueries } = useSyndicateItemQueries({
    queryData,
    isActive,
  });

  // Mutations
  const { updateMutation, importMutation, updateMultipleMutation, exportMutation, sellStockMutation, deleteMutation, deleteMultipleMutation } =
    useMutations({
      refetchQueries,
      setLoadingRows,
    });
  // Modals
  const { OpenMinimumPriceModal, OpenDeleteMultipleModal, OpenUpdateMultipleModal, OpenSellModal, OpenInfoModal, OpenDeleteModal } = useModals({
    updateMutation,
    updateMultipleMutation,
    sellStockMutation,
    deleteMutation,
    deleteMultipleMutation,
  });
  const handleRefresh = (data: { id: string }) => {
    if (data.id) setSelectedRecords((prev) => prev.filter((record) => record.id !== Number(data.id)));
    refetchQueries(true);
  };

  useEffect(() => {
    setSelectedRecords([]);
  }, [deleteMultipleMutation.isSuccess, deleteMutation.isSuccess]);
  useEffect(() => {
    if (!paginationQuery.data) return;
    const results = paginationQuery.data.results || [];
    const resultsById = new Map(results.map((record) => [record.id, record]));
    setSelectedRecords((prev) => {
      if (prev.length === 0) return prev;
      const next = prev.map((record) => resultsById.get(record.id)).filter((record): record is TauriTypes.SyndicateItem => Boolean(record));
      if (next.length === prev.length && next.every((record, index) => record === prev[index])) return prev;
      return next;
    });
  }, [paginationQuery.data]);

  const hasOverride = (record: TauriTypes.SyndicateItem) => {
    return record.properties?.min_sma != null || record.properties?.min_profit != null;
  };

  // Use the custom hook for Tauri events
  useTauriEvent(TauriTypes.Events.RefreshSyndicateItems, handleRefresh, [refetchQueries]);
  return (
    <Box>
      <Grid>
        <Grid.Col span={9}>
          <Group gap={"md"} mt={"md"}>
            {Object.entries(statusCountsQuery.data || {})
              .sort(([a], [b]) => a.localeCompare(b))
              .map(([key, count]) => (
                <ColorInfo
                  active={key == queryData.status}
                  key={key}
                  onClick={() =>
                    setQueryData((prev) => ({
                      ...prev,
                      status: (key as TauriTypes.StockStatus) == prev.status ? undefined : (key as TauriTypes.StockStatus),
                    }))
                  }
                  infoProps={{
                    "data-color-mode": "bg",
                    "data-stock-status": key,
                  }}
                  text={useTranslateStockStatus(`${key}`) + ` (${count})`}
                  tooltip={useTranslateStockStatus(`details.${key}`)}
                />
              ))}
            {Object.entries(syndicateCountsQuery.data || {})
              .sort(([a], [b]) => a.localeCompare(b))
              .map(([key, count]) => (
                <ColorInfo
                  active={key == queryData.syndicate}
                  key={key}
                  onClick={() =>
                    setQueryData((prev) => ({
                      ...prev,
                      syndicate: key == prev.syndicate ? undefined : key,
                    }))
                  }
                  infoProps={{
                    "data-color-mode": "bg",
                    "data-stock-status": key,
                    style: { backgroundColor: GetSyndicateColor(key) },
                  }}
                  text={`${key} (${count})`}
                  tooltip={useTranslateStockStatus(`details.${key}`)}
                />
              ))}
          </Group>
          <Group gap={"md"} mt={"md"}></Group>
        </Grid.Col>
        <Grid.Col span={3}>
          <StatsWithSegments
            showPercent
            percentSymbol="%"
            segments={[{ label: useTranslateSegments("listed"), count: financialReportQuery.data?.revenue || 0, color: "var(--qf-positive-color)" }]}
          />
        </Grid.Col>
      </Grid>
      <SearchField
        value={queryData.query || ""}
        onChange={(value) => setQueryData((prev) => ({ ...prev, query: value }))}
        rightSectionWidth={30 * 4}
        rightSection={
          <Group gap={3}>
            <ActionWithTooltip
              tooltip={useTranslateTabItem("buttons.import_items_tooltip")}
              icon={faPlay}
              color="green.7"
              iconProps={{ size: "xs" }}
              actionProps={{ size: "sm" }}
              onClick={() => importMutation.mutate({})}
            />
            <ActionWithTooltip
              tooltip={useTranslate("export_json_tooltip")}
              icon={faDownload}
              iconProps={{ size: "xs" }}
              actionProps={{ size: "sm", disabled: !canExport }}
              onClick={() => exportMutation.mutate(queryData)}
            />
            <ActionWithTooltip
              tooltip={useTranslate("update_multiple_tooltip")}
              icon={faEdit}
              iconProps={{ size: "xs" }}
              actionProps={{ size: "sm", disabled: selectedRecords.length === 0 }}
              onClick={() => OpenUpdateMultipleModal(selectedRecords.map((r) => r.id))}
            />
            <ActionWithTooltip
              tooltip={useTranslate("delete_multiple_tooltip")}
              icon={faTrashCan}
              color="red.7"
              iconProps={{ size: "xs" }}
              actionProps={{ size: "sm", disabled: selectedRecords.length === 0 }}
              onClick={() => OpenDeleteMultipleModal(selectedRecords.map((r) => r.id))}
            />
          </Group>
        }
      />
      <DataTable
        className={`${classes.databaseSyndicateItems} ${useHasAlert() ? classes.alert : ""} ${is_running ? classes.running : ""}`}
        customRowAttributes={(record) => {
          return {
            "data-color-mode": "box-shadow",
            "data-stock-status": record.status,
          };
        }}
        mt={"md"}
        striped
        fetching={paginationQuery.isLoading}
        records={paginationQuery.data?.results || []}
        page={getSafePage(queryData.page, paginationQuery.data?.total_pages)}
        onPageChange={(page) => setQueryData((prev) => ({ ...prev, page }))}
        totalRecords={paginationQuery.data?.total || 0}
        recordsPerPage={queryData.limit || 10}
        recordsPerPageOptions={[5, 10, 15, 20, 25, 50, 100]}
        onRecordsPerPageChange={(limit) => setQueryData((prev) => ({ ...prev, limit }))}
        sortStatus={{
          columnAccessor: queryData.sort_by || "name",
          direction: queryData.sort_direction || "desc",
        }}
        onSortStatusChange={(sort) => {
          if (!sort || !sort.columnAccessor) return;
          setQueryData((prev) => ({ ...prev, sort_by: sort.columnAccessor as string, sort_direction: sort.direction }));
        }}
        onCellClick={({ record, column }) => {
          switch (column.accessor) {
            case "item_name":
              let name = record.item_name;
              navigator.clipboard.writeText(name);
              notifications.show({
                title: useTranslateCommon("notifications.copy_to_clipboard.title"),
                message: useTranslateCommon("notifications.copy_to_clipboard.message", { message: name }),
                color: "green.7",
              });
              break;
          }
        }}
        selectedRecords={selectedRecords}
        onSelectedRecordsChange={setSelectedRecords}
        // define columns
        columns={[
          {
            accessor: "item_name",
            title: useTranslateCommon("item_name.title"),
            sortable: true,
            render: (row) => <ItemName color="gray.4" size="md" value={row} />,
          },
          {
            accessor: "syndicate_name",
            title: useTranslateDataGridColumns("syndicate_name"),
            sortable: true,
          },
          {
            accessor: "standing_cost",
            title: useTranslateDataGridColumns("standing_cost"),
            sortable: true,
            render: ({ standing_cost }) => <NumberFormatter thousandSeparator="." decimalSeparator="," value={standing_cost} />,
          },
          {
            accessor: "minimum_price",
            width: 310,
            sortable: true,
            title: useTranslateCommon("datatable_columns.minimum_price.title"),
            render: ({ id, properties, list_price }) => (
              <ColumnMinMaxPrice
                id={id}
                minimum_price={properties?.min_price}
                onUpdate={async (id, min_price) => await updateMutation.mutateAsync({ id, properties: { min_price }, list_price })}
                onEdit={async (id, min_price) => OpenMinimumPriceModal(id, min_price)}
              />
            ),
          },
          {
            accessor: "list_price",
            sortable: true,
            title: useTranslateCommon("datatable_columns.list_price"),
          },
          {
            accessor: "actions",
            title: useTranslateCommon("datatable_columns.actions.title"),
            width: 215,
            render: (row) => (
              <ColumnActions
                row={row}
                buttonProps={{ edit: { color: hasOverride(row) ? "yellow.7" : "blue.7" } }}
                hideButtons={["open_filter"]}
                loadingRows={loadingRows}
                onManual={() => OpenSellModal(row)}
                onAuto={(price) => sellStockMutation.mutateAsync({ ...row, price, rawSyndicate: row.syndicate_unique_name })}
                onInfo={() => OpenInfoModal(row)}
                onDelete={() => OpenDeleteModal(row.id)}
                onEdit={() => OpenUpdateMultipleModal([row.id])}
              />
            ),
          },
        ]}
      />
    </Box>
  );
};
