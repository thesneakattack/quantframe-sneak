import { TauriTypes } from "$types";
import { ItemDetailsModal, Operations } from "@components/Modals/ItemDetails";
import { useTranslateCommon } from "@hooks/useTranslate.hook";
import { Text } from "@mantine/core";
import { modals } from "@mantine/modals";

interface ModalHooks {
  updateMutation: {
    mutateAsync: (data: TauriTypes.UpdateSyndicateItem) => Promise<any>;
  };
  updateMultipleMutation: {
    mutateAsync: (data: { ids: number[]; input: TauriTypes.UpdateSyndicateItem }) => Promise<any>;
  };
  sellStockMutation: {
    mutateAsync: (data: TauriTypes.SellSyndicateItem) => Promise<any>;
  };
  deleteMutation: {
    mutateAsync: (id: number) => Promise<any>;
  };
  deleteMultipleMutation: {
    mutateAsync: (ids: number[]) => Promise<any>;
  };
}

export const useModals = ({ updateMutation, updateMultipleMutation, sellStockMutation, deleteMutation, deleteMultipleMutation }: ModalHooks) => {
  const OpenMinimumPriceModal = (id: number, minimum_price: number) => {
    modals.openContextModal({
      modal: "prompt",
      title: useTranslateCommon("prompts.minimum_price.title"),
      innerProps: {
        fields: [
          {
            name: "min_price",
            label: useTranslateCommon("prompts.minimum_price.fields.minimum_price.label"),
            attributes: {
              min: 0,
              description: useTranslateCommon("prompts.minimum_price.fields.minimum_price.description"),
            },
            value: minimum_price,
            type: "number",
          },
        ],
        onConfirm: async (data: { min_price: number | undefined }) => {
          if (!id) return;
          let { min_price } = data;
          await updateMutation.mutateAsync({ id, properties: { min_price } });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  const OpenSellModal = (stock: TauriTypes.SyndicateItem) => {
    modals.openContextModal({
      modal: "prompt",
      title: useTranslateCommon("prompts.sell_manual.title"),
      innerProps: {
        fields: [
          {
            name: "sell",
            label: useTranslateCommon("prompts.sell_manual.fields.sell.label"),
            attributes: {
              min: 0,
            },
            value: 0,
            type: "number",
          },
        ],
        onConfirm: async (data: { sell: number }) => {
          if (!stock) return;
          const { sell } = data;
          await sellStockMutation.mutateAsync({
            id: stock.id,
            wfm_url: stock.wfm_url,
            rawSyndicate: stock.syndicate_unique_name,
            sub_type: stock.sub_type,
            price: sell,
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  const OpenInfoModal = (item: TauriTypes.SyndicateItem) => {
    modals.open({
      size: "100%",
      withCloseButton: false,
      children: (
        <ItemDetailsModal
          value={item.id}
          lookup="syndicate_item"
          operations={[Operations.ProfitabilityInfo, Operations.MarketInfo, Operations.TransactionInfo]}
        />
      ),
    });
  };

  const OpenUpdateMultipleModal = (ids: number[]) => {
    let id = modals.open({
      size: "100%",
      withCloseButton: false,
      children: (
        <ItemDetailsModal
          value={ids[0]}
          lookup="syndicate_item"
          operations={[Operations.EditForm]}
          onSave={async (input: TauriTypes.UpdateSyndicateItem) => {
            if (ids.length == 1) await updateMutation.mutateAsync(input);
            else await updateMultipleMutation.mutateAsync({ ids, input: { ...input, id: -1 } });
            modals.close(id);
          }}
        />
      ),
    });
  };

  const OpenDeleteModal = (id: number) => {
    modals.openConfirmModal({
      title: useTranslateCommon("prompts.delete_item.title"),
      children: <Text size="sm">{useTranslateCommon("prompts.delete_item.message", { count: 1 })}</Text>,
      labels: { confirm: useTranslateCommon("prompts.delete_item.confirm"), cancel: useTranslateCommon("prompts.delete_item.cancel") },
      onConfirm: async () => await deleteMutation.mutateAsync(id),
    });
  };

  const OpenDeleteMultipleModal = (ids: number[]) => {
    modals.openConfirmModal({
      title: useTranslateCommon("prompts.delete_multiple_items.title"),
      children: <Text size="sm">{useTranslateCommon("prompts.delete_multiple_items.message", { count: ids.length })}</Text>,
      labels: {
        confirm: useTranslateCommon("prompts.delete_multiple_items.confirm"),
        cancel: useTranslateCommon("prompts.delete_multiple_items.cancel"),
      },
      onConfirm: async () => await deleteMultipleMutation.mutateAsync(ids),
    });
  };

  return {
    OpenMinimumPriceModal,
    OpenSellModal,
    OpenInfoModal,
    OpenUpdateMultipleModal,
    OpenDeleteModal,
    OpenDeleteMultipleModal,
  };
};
