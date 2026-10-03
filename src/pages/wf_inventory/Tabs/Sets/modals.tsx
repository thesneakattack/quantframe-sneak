import { modals } from "@mantine/modals";
import { TauriTypes } from "$types";
import { useTranslateCommon } from "@hooks/useTranslate.hook";

interface ModalHooks {
  createMutation: {
    mutateAsync: (data: TauriTypes.CreateStockItem) => Promise<any>;
  };
}

export const useModals = ({ createMutation }: ModalHooks) => {
  const OpenAddToStockModal = (set: TauriTypes.WFInvSet) => {
    // Only complete sets reach here, so complete_copies is at least 1.
    const copies = set.complete_copies;
    modals.openContextModal({
      modal: "prompt",
      title: set.name,
      innerProps: {
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
            attributes: { min: 1, max: copies },
            value: copies,
            type: "number",
          },
        ],
        onConfirm: async (data: { bought: number; quantity: number }) => {
          await createMutation.mutateAsync({
            raw: set.wfm_url,
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
