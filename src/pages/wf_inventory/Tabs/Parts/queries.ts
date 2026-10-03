import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import { TauriTypes } from "$types";

interface QueriesHooks {
  queryData: TauriTypes.WFItemControllerGetListParams;
  isActive?: boolean;
}

export const useQueries = ({ queryData, isActive }: QueriesHooks) => {
  const getPartsQuery = useQuery({
    queryKey: ["wf_inventory_get_parts", queryData],
    queryFn: () => api.wf_inventory.getPartsPagination(queryData),
    retry: false,
    enabled: isActive,
  });
  const refetchQueries = () => {
    getPartsQuery.refetch();
  };
  return {
    partsQuery: getPartsQuery,
    refetchQueries,
  };
};
