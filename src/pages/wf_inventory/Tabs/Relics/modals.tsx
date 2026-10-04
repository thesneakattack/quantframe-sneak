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
  // Listing is per refinement, not per relic: an Intact and a Radiant are
  // different products on the market and routinely differ by an order of
  // magnitude, so the variant is carried through to the stock item.
  //
  // The refinement is bound as `tier` because eslint's react-hooks/refs rule
  // treats any identifier containing "ref" as a React ref and reports every
  // read of it as a ref access during render. Nothing here is a ref.
  const OpenAddToStockModal = (relic: TauriTypes.WFInvRelic, tier: TauriTypes.WFInvRelicRefinement) => {
    const owned = tier.owned || 1;
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

    modals.openContextModal({
      modal: "prompt",
      title: `${relic.name} (${tier.variant})`,
      innerProps: {
        fields,
        onConfirm: async (data: { bought: number; quantity: number }) => {
          await createMutation.mutateAsync({
            raw: relic.wfm_url,
            bought: data.bought,
            quantity: data.quantity,
            sub_type: { variant: tier.variant },
          });
        },
        onCancel: (id: string) => modals.close(id),
      },
    });
  };

  return { OpenAddToStockModal };
};
