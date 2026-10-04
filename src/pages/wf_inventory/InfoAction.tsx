import { TauriTypes } from "$types";
import { ItemDetailsModal, Operations } from "@components/Modals/ItemDetails";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faInfo } from "@fortawesome/free-solid-svg-icons";
import { useTranslatePages } from "@hooks/useTranslate.hook";
import { modals } from "@mantine/modals";

interface InventoryInfoActionProps {
  wfmUrl: string;
  subType?: TauriTypes.SubType;
}

/// Shared by every inventory tab so the button means one thing everywhere, and
/// so a change to what "info" shows happens in one place rather than five.
export const InventoryInfoAction = ({ wfmUrl, subType }: InventoryInfoActionProps) => {
  const tooltip = useTranslatePages("wf_inventory.info_tooltip");
  return (
    <ActionWithTooltip
      tooltip={tooltip}
      icon={faInfo}
      actionProps={{ size: "sm", variant: "subtle" }}
      iconProps={{ size: "xs" }}
      onClick={(e: React.MouseEvent) => {
        // Rows are expandable on some tabs; opening the panel must not also
        // toggle the row underneath it.
        e.stopPropagation();
        modals.open({
          size: "100%",
          withCloseButton: false,
          children: (
            <ItemDetailsModal
              value={wfmUrl}
              subType={subType}
              lookup="inventory_item"
              operations={[Operations.MarketInfo, Operations.TransactionInfo]}
            />
          ),
        });
      }}
    />
  );
};
