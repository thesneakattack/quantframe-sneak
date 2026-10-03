import { Group, Box, TextInput, Collapse, Divider } from "@mantine/core";
import { useTranslateComponent } from "@hooks/useTranslate.hook";
import { faAdd, faFilter, faSearch } from "@fortawesome/free-solid-svg-icons";
import { useDebouncedCallback, useToggle } from "@mantine/hooks";
import { useEffect, useState } from "react";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";

export type SearchFieldProps = {
  value: string;
  onChange: (text: string) => void;
  onSearch?: (text: string) => void;
  hideSearch?: boolean;
  searchDisabled?: boolean;
  description?: string;
  onCreate?: () => void;
  rightSection?: React.ReactNode;
  rightSectionWidth?: number;
  filter?: React.ReactNode;
  onFilterToggle?: (open: boolean) => void;
  /**
   * Milliseconds to wait after the last keystroke before calling `onChange`.
   *
   * Opt-in: without it `onChange` fires per keystroke, as it always has. Pass
   * it where each change costs a round trip - the WF Inventory tabs rebuild a
   * projection over the whole inventory and take the same cache mutex the
   * live scraper holds, so one query per pause instead of one per character
   * matters there.
   */
  debounce?: number;
};
export function SearchField({
  value,
  filter,
  description,
  onSearch,
  searchDisabled,
  onCreate,
  onChange,
  hideSearch,
  rightSection,
  onFilterToggle,
  rightSectionWidth,
  debounce,
}: SearchFieldProps) {
  // States
  const [openFilter, setOpenFilter] = useToggle();
  const [sectionWidth, setSectionWidth] = useState(115);
  // What the box shows. Kept locally so typing stays instant even while the
  // emit to the parent is held back.
  const [draft, setDraft] = useState(value);

  const emitDebounced = useDebouncedCallback(onChange, debounce ?? 0);
  // Without a debounce, emit synchronously exactly as before - routing through
  // a zero-delay timeout would make every existing caller asynchronous.
  const emit = debounce ? emitDebounced : onChange;

  // Follow the parent when it changes the value from outside, for instance a
  // reset or a restored query. When the parent is merely catching up to what
  // we emitted, value already equals the draft and this does nothing.
  useEffect(() => {
    setDraft((current) => (value === current ? current : value));
  }, [value]);

  // Translate general
  const useTranslateSearchField = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateComponent(`searchfield.${key}`, { ...context }, i18Key);
  const useTranslateSearchFieldButtons = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateComponent(`searchfield.buttons.${key}`, { ...context }, i18Key);

  // On change
  useEffect(() => {
    const buttonWidth = 35;
    let width = 0;
    if (filter) width += buttonWidth;
    if (onSearch) width += buttonWidth;
    if (onCreate) width += buttonWidth;
    setSectionWidth(width);
  }, [onChange, onSearch, onCreate, filter]);

  useEffect(() => {
    if (onFilterToggle) onFilterToggle(openFilter);
  }, [openFilter]);

  return (
    <Box>
      <TextInput
        value={draft}
        onKeyDown={(event) => {
          if (event.key !== "Enter") return;
          // Enter means "now", so do not make the user wait out the debounce.
          emitDebounced.flush();
          if (onSearch) onSearch(draft);
        }}
        onChange={(event) => {
          setDraft(event.currentTarget.value);
          emit(event.currentTarget.value);
        }}
        label={useTranslateSearchField("label")}
        placeholder={useTranslateSearchField("placeholder")}
        description={description}
        rightSectionWidth={rightSectionWidth ?? sectionWidth}
        rightSection={
          <Group p={0} m={0} gap={5}>
            <Divider orientation="vertical" />
            {rightSection}
            {filter && (
              <ActionWithTooltip
                tooltip={useTranslateSearchFieldButtons("filter.tooltip")}
                icon={faFilter}
                color={openFilter ? "blue.7" : "dark.4"}
                actionProps={{ size: "sm" }}
                iconProps={{ size: "xs" }}
                onClick={async () => setOpenFilter()}
              />
            )}
            {onSearch && !hideSearch && (
              <ActionWithTooltip
                tooltip={useTranslateSearchFieldButtons("search.tooltip")}
                icon={faSearch}
                color={"blue.7"}
                actionProps={{ size: "sm", disabled: searchDisabled ?? false }}
                iconProps={{ size: "xs" }}
                onClick={async () => {
                  emitDebounced.flush();
                  if (onSearch) onSearch(draft);
                }}
              />
            )}
            {onCreate && (
              <ActionWithTooltip
                tooltip={useTranslateSearchFieldButtons("create.tooltip")}
                icon={faAdd}
                color={"green"}
                actionProps={{ size: "sm" }}
                iconProps={{ size: "xs" }}
                onClick={async () => onCreate()}
              />
            )}
          </Group>
        }
      />
      <Collapse expanded={openFilter}>{filter}</Collapse>
    </Box>
  );
}

