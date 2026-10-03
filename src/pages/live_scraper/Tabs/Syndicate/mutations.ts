import { TauriTypes } from "$types";
import api from "@api/index";
import { createGenericMutation, MutationHooks } from "@utils/genericMutation.helper";

export const useMutations = ({ refetchQueries, setLoadingRows }: MutationHooks) => {
  const hooks = { refetchQueries, setLoadingRows, refetchStatusString: ["import_syndicate_items", "delete_syndicate_item"] };
  const exportMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.SyndicateItemControllerGetListParams) => api.syndicate_item.exportJson(data),
      successKey: "export_data",
      errorKey: "export_data",
      getSuccessMessage: (data: any) => ({ path: data }),
    },
    hooks,
  );

  const updateMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.UpdateSyndicateItem) => api.syndicate_item.update(data),
      successKey: "update_syndicate_item",
      errorKey: "update_syndicate_item",
      getLoadingId: (variables: TauriTypes.UpdateSyndicateItem) => `${variables.id}`,
      getSuccessMessage: (data: any) => ({ name: data.item_name }),
    },
    hooks,
  );

  const updateMultipleMutation = createGenericMutation(
    {
      mutationFn: (data: { ids: number[]; input: TauriTypes.UpdateSyndicateItem }) => api.syndicate_item.updateMultiple(data.ids, data.input),
      successKey: "update_syndicate_item",
      errorKey: "update_syndicate_item",
      isMultiple: (variables: { ids: number[]; input: TauriTypes.UpdateSyndicateItem }) => variables.ids.length > 1,
      getLoadingId: (variables: { ids: number[]; input: TauriTypes.UpdateSyndicateItem }) => variables.ids.map((id) => `${id}`),
      getSuccessMessage: (data: any) => ({ count: data.length }),
    },
    hooks,
  );

  const sellStockMutation = createGenericMutation(
    {
      mutationFn: (data: TauriTypes.SellSyndicateItem) => api.syndicate_item.sell(data),
      successKey: "sell_syndicate_item",
      errorKey: "sell_syndicate_item",
      getLoadingId: (variables: TauriTypes.SellSyndicateItem) => `${variables.id}`,
      getSuccessMessage: (data: any) => ({ name: data.item_name }),
    },
    hooks,
  );

  const deleteMutation = createGenericMutation(
    {
      mutationFn: (id: number) => api.syndicate_item.delete(id),
      successKey: "delete_syndicate_item",
      errorKey: "delete_syndicate_item",
      getLoadingId: (variables: number) => `${variables}`,
      getSuccessMessage: (data: any) => ({ name: data.item_name }),
    },
    hooks,
  );

  const deleteMultipleMutation = createGenericMutation(
    {
      mutationFn: (ids: number[]) => api.syndicate_item.deleteMultiple(ids),
      successKey: "delete_syndicate_item",
      errorKey: "delete_syndicate_item",
      isMultiple: (variables: number[]) => variables.length > 1,
      getLoadingId: (variables: number[]) => variables.map((id) => `${id}`),
      getSuccessMessage: (data: any) => ({ count: data }),
    },
    hooks,
  );

  const importMutation = createGenericMutation(
    {
      mutationFn: () => api.syndicate_item.importItems(),
      successKey: "import_syndicate_items",
      errorKey: "import_syndicate_items",
      getSuccessMessage: (data: number) => ({ count: data }),
    },
    hooks,
  );
  return {
    exportMutation,
    updateMutation,
    updateMultipleMutation,
    sellStockMutation,
    deleteMutation,
    deleteMultipleMutation,
    importMutation,
  };
};
