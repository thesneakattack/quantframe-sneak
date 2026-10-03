import { TauriTypes } from "$types";
import api from "@api/index";
import { GenericItemList } from "@components/Forms/GenericItemList";
import { TokenSearchSelect } from "@components/Forms/TokenSearchSelect";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { ButtonIntervals } from "@components/Shared/ButtonIntervals";
import { TooltipIcon } from "@components/Shared/TooltipIcon";
import { faCheck, faEdit, faFileImport, faPlus, faTrashCan, faXmark } from "@fortawesome/free-solid-svg-icons";
import { useTranslateCommon, useTranslateForms } from "@hooks/useTranslate.hook";
import { Box, Button, Group, Modal, NumberInput, Stack, Text } from "@mantine/core";
import { useForm, UseFormReturnType } from "@mantine/form";
import { useDisclosure } from "@mantine/hooks";
import { useQuery } from "@tanstack/react-query";
import { Operator, OperatorType, type FieldFilter } from "@utils/filter.helper";
import { useState } from "react";

enum ViewMode {
  General = "general",
  Syndicates = "syndicates",
}

export type SyndicatePanelProps = {
  form: UseFormReturnType<TauriTypes.Settings>;
  setHideTab?: (value: boolean) => void;
  setHideButtons?: (value: boolean) => void;
};

export const SyndicatePanel = ({ form, setHideTab, setHideButtons }: SyndicatePanelProps) => {
  // States
  const [viewMode, setViewMode] = useState<ViewMode>(ViewMode.General);
  const [createModalOpened, { open: openCreateModal, close: closeCreateModal }] = useDisclosure(false);
  const filterForm = useForm({ initialValues: { query: "" } });
  const createForm = useForm({ initialValues: { edit: false, unique_name: "", standing: 0 } });

  // Translate general
  const useTranslateForm = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateForms(`settings.tabs.live_scraper.syndicate.wts.${key}`, { ...context }, i18Key);
  const useTranslateFormFields = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateForm(`fields.${key}`, { ...context }, i18Key);
  const useTranslateDatatableColumns = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateForm(`datatable.columns.${key}`, { ...context }, i18Key);
  const useTranslateButtons = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateForm(`buttons.${key}`, { ...context }, i18Key);

  // Fetch data from rust side
  const { data } = useQuery({
    queryKey: ["cache_syndicates"],
    queryFn: () => api.cache.getSyndicates(),
  });

  const getFieldPath = (field: string) => `live_scraper.syndicate.wts.${field}`;

  // Functions
  const GetFilter = () => {
    const filters: FieldFilter[] = [];
    if (filterForm.values.query && filterForm.values.query.trim() !== "") {
      filters.push({
        name: {
          type: OperatorType.STRING,
          [Operator.MATCHES]: filterForm.values.query, // regex-like match
          isCaseSensitive: false,
        },
      });
    }
    return {
      AND: filters, // All must match
    };
  };

  const handleImport = async () => {
    let items = await api.wf_inventory.getSyndicatesPagination({ page: 1, limit: -1, properties: { can_select: true } });
    let syndicates = [...(form.values.live_scraper.syndicate.wts.syndicates || [])];
    for (let item of items.results || []) {
      let index = syndicates.findIndex((s) => s.unique_name === item.unique_name);
      const syndicateInfo = data?.find((s) => s.uniqueName === item.unique_name);
      if (index !== -1) syndicates[index] = { ...syndicates[index], standing: item.quantity || 0 };
      else
        syndicates.push({
          name: syndicateInfo?.name || item.name || item.unique_name,
          standing: item.quantity || 0,
          unique_name: item.unique_name,
          ignore_standing: false,
        });
    }
    form.setFieldValue("live_scraper.syndicate.wts.syndicates", syndicates);
  };

  const handleDelete = (unique_name: string) => {
    let syndicates = form.values.live_scraper.syndicate.wts.syndicates?.filter((item) => item.unique_name !== unique_name) ?? [];
    form.setFieldValue("live_scraper.syndicate.wts.syndicates", syndicates);
  };
  const handleAdd = (unique_name: string, standing: number) => {
    if (!unique_name || unique_name.trim() === "" || standing < 0) return;
    let syndicates = [...(form.values.live_scraper.syndicate.wts.syndicates || [])];
    let index = syndicates.findIndex((s) => s.unique_name === unique_name);
    const syndicateInfo = data?.find((s) => s.uniqueName === unique_name);
    if (index !== -1) syndicates[index] = { ...syndicates[index], standing: standing };
    else syndicates.push({ name: syndicateInfo?.name || unique_name, standing: standing, unique_name: unique_name, ignore_standing: false });
    form.setFieldValue("live_scraper.syndicate.wts.syndicates", syndicates);
  };
  return (
    <Box h="100%" p="md">
      <Modal zIndex={299} opened={createModalOpened} onClose={closeCreateModal} centered title={useTranslateForm("add_syndicate_prompt.title")}>
        <Group grow gap={"md"}>
          <TokenSearchSelect
            disabled={createForm.values.edit}
            label={useTranslateForm("add_syndicate_prompt.syndicate_label")}
            placeholder={useTranslateForm("add_syndicate_prompt.syndicate_placeholder")}
            searchable
            clearable
            autoSelectOnBlur
            selectFirstOptionOnChange
            w={"50%"}
            limit={5}
            value={createForm.values.unique_name}
            onChange={(event) => createForm.setFieldValue("unique_name", event || "")}
            data={data?.filter((item) => item.canSelect)?.map((item) => ({ value: item.uniqueName, label: item.name })) || []}
          />
          <NumberInput
            label={useTranslateForm("add_syndicate_prompt.standing_label")}
            placeholder={useTranslateForm("add_syndicate_prompt.standing_placeholder")}
            radius="md"
            w={"50%"}
            min={0}
            value={createForm.values.standing}
            onChange={(value) => createForm.setFieldValue("standing", Number(value) || 0)}
            rightSectionWidth={35}
            rightSection={
              <ActionWithTooltip
                tooltip={useTranslateForm("add_syndicate_prompt.add_syndicate_label")}
                color={"green.7"}
                icon={faPlus}
                actionProps={{ size: "sm" }}
                iconProps={{ size: "xs" }}
                onClick={() => {
                  handleAdd(createForm.values.unique_name, createForm.values.standing);
                  closeCreateModal();
                }}
              />
            }
          />
        </Group>
      </Modal>
      {viewMode == ViewMode.General && (
        <Stack>
          <Group gap={"md"}>
            <NumberInput
              label={useTranslateFormFields("volume_threshold.label")}
              min={-1}
              placeholder={useTranslateFormFields("volume_threshold.placeholder")}
              rightSection={
                <TooltipIcon label={useTranslateFormFields("volume_threshold.tooltip")} link={useTranslateFormFields("volume_threshold.link")} />
              }
              radius="md"
              {...form.getInputProps(getFieldPath("volume_threshold"))}
            />
            <NumberInput
              label={useTranslateFormFields("max_price_drop.label")}
              min={-1}
              placeholder={useTranslateFormFields("max_price_drop.placeholder")}
              rightSection={
                <TooltipIcon label={useTranslateFormFields("max_price_drop.tooltip")} link={useTranslateFormFields("max_price_drop.link")} />
              }
              radius="md"
              {...form.getInputProps(getFieldPath("max_price_drop"))}
            />
            <NumberInput
              label={useTranslateFormFields("min_listings_below.label")}
              min={-1}
              placeholder={useTranslateFormFields("min_listings_below.placeholder")}
              rightSection={
                <TooltipIcon label={useTranslateFormFields("min_listings_below.tooltip")} link={useTranslateFormFields("min_listings_below.link")} />
              }
              radius="md"
              {...form.getInputProps(getFieldPath("min_listings_below"))}
            />
          </Group>
          <Button
            onClick={() => {
              setHideTab && setHideTab(true);
              setHideButtons && setHideButtons(true);
              setViewMode(ViewMode.Syndicates);
            }}
          >
            {useTranslateForm("buttons.edit_syndicates_label", { count: form.values.live_scraper.syndicate.wts.syndicates?.length || 0 })}
          </Button>
        </Stack>
      )}
      {viewMode == ViewMode.Syndicates && (
        <Stack>
          <GenericItemList
            idAccessor="unique_name"
            searchable
            searchValue={filterForm.values.query}
            onSearchChange={(query) => filterForm.setFieldValue("query", query)}
            searchRightSectionWidth={35 * 3}
            searchRightSection={
              <Group gap={5}>
                <ActionWithTooltip
                  tooltip={useTranslateButtons("clear_tooltip")}
                  color={"red.7"}
                  icon={faTrashCan}
                  actionProps={{ size: "sm" }}
                  iconProps={{ size: "xs" }}
                  onClick={() => form.setFieldValue("live_scraper.syndicate.wts.syndicates", [])}
                />
                <ActionWithTooltip
                  tooltip={useTranslateButtons("import_tooltip")}
                  icon={faFileImport}
                  actionProps={{ size: "sm", disabled: form.values.wf_inventory.source === "None" }}
                  iconProps={{ size: "xs" }}
                  onClick={handleImport}
                />
              </Group>
            }
            onCreate={() => {
              createForm.setFieldValue("edit", false);
              openCreateModal();
            }}
            filter={GetFilter()}
            columns={[
              {
                accessor: "name",
                title: useTranslateDatatableColumns("name"),
              },
              {
                accessor: "standing",
                width: 375,
                title: useTranslateDatatableColumns("standing"),
                render: (item) => (
                  <Group gap={"sm"} justify="space-between">
                    <Text>{item.standing || "N/A"}</Text>
                    <Group gap={"xs"}>
                      <ButtonIntervals
                        disabled={item.ignore_standing}
                        intervals={[100, 200]}
                        minimum_price={item.standing || 0}
                        OnClick={async (val) => {
                          if (!item.unique_name) return;
                          handleAdd(item.unique_name, val);
                        }}
                      />
                      <ActionWithTooltip
                        tooltip={useTranslateButtons("edit_standing_label")}
                        icon={faEdit}
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!item.unique_name) return;
                          openCreateModal();
                          createForm.setFieldValue("edit", true);
                          createForm.setFieldValue("unique_name", item.unique_name);
                          createForm.setFieldValue("standing", item.standing || 0);
                        }}
                        actionProps={{ size: "sm", disabled: item.ignore_standing }}
                        iconProps={{ size: "xs" }}
                      />
                    </Group>
                  </Group>
                ),
              },
              {
                accessor: "actions",
                title: useTranslateCommon("datatable_columns.actions.title"),
                width: 75,
                render: (item) => (
                  <Group gap={5}>
                    <ActionWithTooltip
                      tooltip={
                        item.ignore_standing
                          ? useTranslateDatatableColumns("actions.buttons.ignore_standing_tooltip_enabled")
                          : useTranslateDatatableColumns("actions.buttons.ignore_standing_tooltip_disabled")
                      }
                      icon={item.ignore_standing ? faCheck : faXmark}
                      color={item.ignore_standing ? "green.7" : "red.7"}
                      actionProps={{ size: "sm" }}
                      iconProps={{ size: "xs" }}
                      onClick={() => {
                        let syndicates =
                          form.values.live_scraper.syndicate.wts.syndicates?.map((s) => {
                            if (s.unique_name === item.unique_name) return { ...s, ignore_standing: !s.ignore_standing };
                            return s;
                          }) ?? [];
                        form.setFieldValue("live_scraper.syndicate.wts.syndicates", syndicates);
                      }}
                    />
                    <ActionWithTooltip
                      tooltip={useTranslateCommon("datatable_columns.actions.buttons.delete_tooltip")}
                      color={"red.7"}
                      icon={faTrashCan}
                      actionProps={{ size: "sm" }}
                      iconProps={{ size: "xs" }}
                      onClick={() => handleDelete(item.unique_name)}
                    />
                  </Group>
                ),
              },
            ]}
            items={form.values.live_scraper.syndicate.wts.syndicates || []}
          />
          <Button
            color="blue"
            variant="light"
            onClick={() => {
              setHideTab && setHideTab(false);
              setHideButtons && setHideButtons(false);
              setViewMode(ViewMode.General);
            }}
          >
            {useTranslateButtons("go_back_label")}
          </Button>
        </Stack>
      )}
    </Box>
  );
};
