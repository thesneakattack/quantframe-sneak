import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getRelicsQuery = useQuery({
    queryKey: ["wf_inventory_get_relics", queryData],
    queryFn: () => api.wf_inventory.getRelicsPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getRelicsQuery.refetch();
  };
  return {
    relicsQuery: getRelicsQuery,
    refetchQueries,
  };
};
