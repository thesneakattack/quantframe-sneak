import { TauriTypes } from "$types";
import api from "@api/index";
import { useQuery } from "@tanstack/react-query";

interface QueriesHooks {
  queryData: TauriTypes.SyndicateItemControllerGetListParams;
  isActive?: boolean;
}

export const useSyndicateItemQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getPaginationQuery = useQuery({
    queryKey: ["get_syndicate_item_pagination", queryData],
    queryFn: () => api.syndicate_item.getPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const getFinancialReportQuery = useQuery({
    queryKey: ["get_syndicate_item_financial_report", queryData],
    queryFn: () => api.syndicate_item.getFinancialReport(queryData),
    retry: false,
  });
  const getStatusCountsQuery = useQuery({
    queryKey: ["get_syndicate_item_status_counts"],
    queryFn: () => api.syndicate_item.getStatusCounts({ page: 1, limit: -1 }),
    retry: false,
    enabled: isActive,
  });
  const getSyndicateCountsQuery = useQuery({
    queryKey: ["get_syndicate_item_syndicate_counts"],
    queryFn: () => api.syndicate_item.getSyndicateCounts({ page: 1, limit: -1 }),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = (refetchStatus: boolean = false) => {
    getPaginationQuery.refetch();
    getFinancialReportQuery.refetch();
    if (refetchStatus) getStatusCountsQuery.refetch();
    if (refetchStatus) getSyndicateCountsQuery.refetch();
  };

  // Return the queries
  return {
    paginationQuery: getPaginationQuery,
    financialReportQuery: getFinancialReportQuery,
    statusCountsQuery: getStatusCountsQuery,
    syndicateCountsQuery: getSyndicateCountsQuery,
    refetchQueries,
  };
};
