import { TauriTypes } from "$types";
import api from "@api/index";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faRotate } from "@fortawesome/free-solid-svg-icons";
import { useTranslatePages } from "@hooks/useTranslate.hook";
import { useState } from "react";

interface RefreshPriceActionProps {
  wfmUrl: string;
  subType?: TauriTypes.SubType;
  /** Null for an item with no ranks, so no second request is made. */
  maxRank?: number | null;
  onRefreshed: () => void;
}

/// Shared by every inventory tab, so "refresh" means one thing everywhere and
/// the behaviour changes in one place rather than five.
export const RefreshPriceAction = ({ wfmUrl, subType, maxRank, onRefreshed }: RefreshPriceActionProps) => {
  const tooltip = useTranslatePages("wf_inventory.refresh_price_tooltip");
  const [loading, setLoading] = useState(false);
  return (
    <ActionWithTooltip
      tooltip={tooltip}
      icon={faRotate}
      actionProps={{ size: "sm", variant: "subtle", loading }}
      iconProps={{ size: "xs" }}
      onClick={async (e: React.MouseEvent) => {
        // Rows expand on some tabs; refreshing must not also toggle the row.
        e.stopPropagation();
        setLoading(true);
        try {
          await api.wf_inventory.refreshPrice(wfmUrl, subType, maxRank);
          onRefreshed();
        } finally {
          // Even a failed fetch has to give the button back, or the row looks
          // permanently busy over a request that is already over.
          setLoading(false);
        }
      }}
    />
  );
};
