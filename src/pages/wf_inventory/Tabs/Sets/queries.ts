import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getSetsQuery = useQuery({
    queryKey: ["wf_inventory_get_sets", queryData],
    queryFn: () => api.wf_inventory.getSetsPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getSetsQuery.refetch();
  };
  return {
    setsQuery: getSetsQuery,
    refetchQueries,
  };
};
