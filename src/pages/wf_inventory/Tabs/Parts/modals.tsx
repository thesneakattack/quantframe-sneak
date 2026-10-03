import { modals } from "@mantine/modals";
import { TauriTypes } from "$types";
import { useTranslateCommon } from "@hooks/useTranslate.hook";

interface ModalHooks {
  createMutation: {
    mutateAsync: (data: TauriTypes.CreateStockItem) => Promise<any>;
  };
}

export const useModals = ({ createMutation }: ModalHooks) => {
  const OpenAddToStockModal = (item: TauriTypes.WFInvItemRow) => {
    const owned = item.quantity || 1;
    const conflicts = item.properties?.in_stock_sets || [];
    modals.openContextModal({
      modal: "prompt",
      title: item.name,
      innerProps: {
        // Advisory only: listing a part whose set is already listed undercuts
        // that set, but confirming is still allowed.
        message: conflicts.length ? useTranslateCommon("prompts.stock_set_conflict.message", { sets: conflicts.join(", ") }) : undefined,
        fields: [
          {
            name: "bought",
            label: useTranslateCommon("prompts.bought_manual.fields.bought.label"),
            attributes: { min: 0 },
            value: 0,
            type: "number",
            autoFocus: true,
          },
          {
            name: "quantity",
            label: useTranslateCommon("prompts.bought_manual.fields.quantity.label"),
            attributes: { min: 1, max: owned },
            value: owned,
            type: "number",
          },
        ],
        onConfirm: async (data: { bought: number; quantity: number }) => {
          await createMutation.mutateAsync({
            raw: item.wfm_url,
            bought: data.bought,
            quantity: data.quantity,
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  return { OpenAddToStockModal };
};
