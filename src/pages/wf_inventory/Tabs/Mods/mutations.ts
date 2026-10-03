import { TauriTypes } from "$types";
import api from "@api/index";
import { createGenericMutation, MutationHooks } from "@utils/genericMutation.helper";
import { inventoryRowKey } from "@utils/helper";

export const useMutations = ({ refetchQueries, setLoadingRows }: MutationHooks) => {
  const hooks = { refetchQueries, setLoadingRows };

  const createMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.CreateStockItem) => api.stock_item.create(data),
      successKey: "create_stock_item",
      errorKey: "create_stock_item",
      // Not data.raw: two rows can share a wfm_url, and they must not
      // share a spinner.
      getLoadingId: (data: TauriTypes.CreateStockItem) => inventoryRowKey(data.raw, data.sub_type),
      getSuccessMessage: (data: TauriTypes.StockItem) => ({ name: data.item_name }),
    },
    hooks,
  );
  return { createMutation };
};
