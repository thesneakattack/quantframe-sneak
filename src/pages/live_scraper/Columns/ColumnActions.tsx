import { TauriTypes } from "$types";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { Countdown } from "@components/Shared/Countdown";
import { faClock, faEdit, faEye, faEyeSlash, faFilter, faHammer, faInfo, faPen, faTrashCan } from "@fortawesome/free-solid-svg-icons";
import { useTranslateCommon } from "@hooks/useTranslate.hook";
import { Group } from "@mantine/core";
import dayjs from "dayjs";
export type ColumnActionsProps = {
  cooldown?: TauriTypes.CoolDownInfo;
  i18nKeyOverride?: Record<string, string>;
  row: { id: number; list_price?: number | undefined; is_hidden?: boolean; properties?: TauriTypes.StockEntryPropertiesBase };
  loadingRows: string[];
  hideButtons?: string[];
  buttonProps?: { [key: string]: any };
  onManual: () => void;
  onAuto: (price: number) => void;
  onInfo: () => void;
  onFilter?: () => void;
  onHide?: (hide: boolean) => void;
  onDelete: (id: number) => void;
  onEdit?: (id: number) => void;
};

export function ColumnActions({
  row,
  loadingRows,
  i18nKeyOverride,
  hideButtons,
  buttonProps,
  onManual,
  onAuto,
  onInfo,
  onFilter,
  onHide,
  onDelete,
  onEdit,
}: ColumnActionsProps) {
  // Functions
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateCommon(`datatable_columns.actions.${key}`, { ...context }, i18Key);
  const useTranslateButtons = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslate(`buttons.${i18nKeyOverride?.[key] || key}`, { ...context }, i18Key);

  const HideButton = (id: string) => {
    if (!hideButtons) return true;
    return !hideButtons.includes(id);
  };

  const showCooldown = row.properties?.cooldown && dayjs().isBefore(dayjs(row.properties.cooldown.end_time));

  return (
    <Group gap={5} justify="flex-end">
      {showCooldown && (
        <ActionWithTooltip
          tooltip={
            <Countdown
              startDate={new Date(row.properties?.cooldown?.start_time || "")}
              endDate={new Date(row.properties?.cooldown?.end_time || "")}
            />
          }
          icon={faClock}
          color={"yellow.7"}
          actionProps={{ size: "sm" }}
          iconProps={{ size: "xs" }}
          onClick={() => {}}
        />
      )}
      <ActionWithTooltip
        tooltip={useTranslateButtons("manual_tooltip")}
        icon={faPen}
        loading={loadingRows.includes(`${row.id}`)}
        color={"green.7"}
        actionProps={{ size: "sm" }}
        iconProps={{ size: "xs" }}
        onClick={(e) => {
          e.stopPropagation();
          onManual();
        }}
      />
      {HideButton("open_filter") && (
        <ActionWithTooltip
          {...(buttonProps?.["open_filter"] || {})}
          tooltip={useTranslateButtons("open_filter_tooltip")}
          icon={faFilter}
          loading={loadingRows.includes(`${row.id}`)}
          actionProps={{ size: "sm" }}
          iconProps={{ size: "xs" }}
          onClick={async (e) => {
            e.stopPropagation();
            onFilter?.();
          }}
        />
      )}
      <ActionWithTooltip
        tooltip={useTranslateButtons("auto_tooltip")}
        icon={faHammer}
        loading={loadingRows.includes(`${row.id}`)}
        actionProps={{ disabled: !row.list_price, size: "sm" }}
        iconProps={{ size: "xs" }}
        onClick={async (e) => {
          e.stopPropagation();
          if (!row.id || !row.list_price) return;
          onAuto(row.list_price);
        }}
      />
      <ActionWithTooltip
        tooltip={useTranslateButtons("info_tooltip")}
        icon={faInfo}
        loading={loadingRows.includes(`${row.id}`)}
        actionProps={{ size: "sm" }}
        iconProps={{ size: "xs" }}
        onClick={(e) => {
          e.stopPropagation();
          onInfo();
        }}
      />
      {onEdit && (
        <ActionWithTooltip
          {...(buttonProps?.["edit"] || {})}
          tooltip={useTranslateButtons("edit_tooltip")}
          icon={faEdit}
          loading={loadingRows.includes(`${row.id}`)}
          actionProps={{ size: "sm" }}
          iconProps={{ size: "xs" }}
          onClick={async (e) => {
            e.stopPropagation();
            onEdit(row.id);
          }}
        />
      )}
      {onHide && (
        <ActionWithTooltip
          tooltip={useTranslateButtons(`hide.${row.is_hidden ? "disabled_tooltip" : "enabled_tooltip"}`)}
          icon={row.is_hidden ? faEyeSlash : faEye}
          loading={loadingRows.includes(`${row.id}`)}
          color={`${row.is_hidden ? "red.7" : "green.7"}`}
          actionProps={{ size: "sm" }}
          iconProps={{ size: "xs" }}
          onClick={async (e) => {
            e.stopPropagation();
            onHide(!row.is_hidden);
          }}
        />
      )}
      <ActionWithTooltip
        tooltip={useTranslateButtons("delete_tooltip")}
        color={"red.7"}
        icon={faTrashCan}
        loading={loadingRows.includes(`${row.id}`)}
        actionProps={{ size: "sm" }}
        iconProps={{ size: "xs" }}
        onClick={async (e) => {
          e.stopPropagation();
          onDelete(row.id);
        }}
      />
    </Group>
  );
}
