import { modals } from "@mantine/modals";
import { TauriTypes } from "$types";
import { PromptField } from "@components/Modals/Prompt";
import { useTranslateCommon } from "@hooks/useTranslate.hook";

interface ModalHooks {
  createMutation: {
    mutateAsync: (data: TauriTypes.CreateStockItem) => Promise<any>;
  };
}

export const useModals = ({ createMutation }: ModalHooks) => {
  const OpenAddToStockModal = (item: TauriTypes.WFInvItemRow) => {
    const owned = item.quantity || 1;
    const maxRank = item.properties?.max_rank ?? null;
    // Prefilled from the inventory row, not from maxRank: the row already
    // knows the real rank, and defaulting to max would misprice the listing.
    const currentRank = item.sub_type?.rank ?? 0;

    const fields: PromptField[] = [
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
    ];
    if (maxRank != null) {
      fields.push({
        name: "rank",
        label: useTranslateCommon("prompts.bought_manual.fields.rank.label"),
        attributes: { min: 0, max: maxRank },
        value: currentRank,
        type: "number",
      });
    }

    modals.openContextModal({
      modal: "prompt",
      title: item.name,
      innerProps: {
        fields,
        onConfirm: async (data: { bought: number; quantity: number; rank?: number }) => {
          await createMutation.mutateAsync({
            raw: item.wfm_url,
            bought: data.bought,
            quantity: data.quantity,
            sub_type: maxRank != null ? { rank: data.rank ?? currentRank } : undefined,
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  return { OpenAddToStockModal };
};
