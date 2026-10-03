import { TauriTypes } from "$types";
import api from "@api/index";
import { createGenericMutation, MutationHooks } from "@utils/genericMutation.helper";

export const useMutations = ({ refetchQueries, setLoadingRows }: MutationHooks) => {
  const hooks = { refetchQueries, setLoadingRows };

  const createMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.CreateStockItem) => api.stock_item.create(data),
      successKey: "create_stock_item",
      errorKey: "create_stock_item",
      getLoadingId: (data: TauriTypes.CreateStockItem) => data.raw,
      getSuccessMessage: (data: TauriTypes.StockItem) => ({ name: data.item_name }),
    },
    hooks,
  );
  return { createMutation };
};
