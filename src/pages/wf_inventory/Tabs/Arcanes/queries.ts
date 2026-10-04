import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getArcanesQuery = useQuery({
    queryKey: ["wf_inventory_get_arcanes", queryData],
    queryFn: () => api.wf_inventory.getArcanesPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getArcanesQuery.refetch();
  };
  return {
    arcanesQuery: getArcanesQuery,
    refetchQueries,
  };
};
