import { TauriTypes } from "$types";
import { ActionWithTooltip } from "@components/Shared/ActionWithTooltip";
import { faArrowDown, faArrowUp, faPlus, faXmark } from "@fortawesome/free-solid-svg-icons";
import { useTranslateComponent } from "@hooks/useTranslate.hook";
import { ActionIcon, Group, Select, Stack, Text } from "@mantine/core";

export type SortColumn = { value: string; label: string };

export type SortPriorityProps = {
  /** Columns the caller can sort by, in menu order. */
  columns: SortColumn[];
  value: TauriTypes.SortField[];
  onChange: (sorts: TauriTypes.SortField[]) => void;
};

/**
 * Edits a sort as an ordered list rather than a single column.
 *
 * The first entry decides, each later one breaks the ties above it, which is
 * what makes a sort on a column like "owned" useful at all: nearly every row
 * shares the same value there, so without a second key the result looks
 * unsorted.
 */
export function SortPriority({ columns, value, onChange }: SortPriorityProps) {
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslateComponent(`sort_priority.${key}`, { ...context }, i18Key);

  const unused = columns.filter((column) => !value.some((sort) => sort.by === column.value));

  const replace = (index: number, next: Partial<TauriTypes.SortField>) =>
    onChange(value.map((sort, i) => (i === index ? { ...sort, ...next } : sort)));

  const move = (index: number, by: number) => {
    const next = [...value];
    const target = index + by;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  return (
    <Stack gap={4}>
      <Text size="sm" fw={500}>
        {useTranslate("label")}
      </Text>
      {value.map((sort, index) => (
        <Group key={sort.by} gap={4} wrap="nowrap">
          <Text size="xs" c="dimmed" w={14}>
            {index + 1}
          </Text>
          <Select
            w={150}
            size="xs"
            allowDeselect={false}
            value={sort.by}
            // Only columns not already in the list, plus this entry's own.
            data={[...unused, columns.find((c) => c.value === sort.by)!].filter(Boolean)}
            onChange={(by) => by && replace(index, { by })}
          />
          <ActionWithTooltip
            tooltip={useTranslate(`direction.${sort.direction === "asc" ? "ascending" : "descending"}`)}
            icon={sort.direction === "asc" ? faArrowUp : faArrowDown}
            actionProps={{ size: "sm", variant: "subtle" }}
            iconProps={{ size: "xs" }}
            onClick={() => replace(index, { direction: sort.direction === "asc" ? "desc" : "asc" })}
          />
          <ActionIcon size="sm" variant="subtle" disabled={index === 0} onClick={() => move(index, -1)}>
            <Text size="xs">↑</Text>
          </ActionIcon>
          <ActionIcon size="sm" variant="subtle" disabled={index === value.length - 1} onClick={() => move(index, 1)}>
            <Text size="xs">↓</Text>
          </ActionIcon>
          <ActionWithTooltip
            tooltip={useTranslate("remove")}
            icon={faXmark}
            color="red.7"
            actionProps={{ size: "sm", variant: "subtle" }}
            iconProps={{ size: "xs" }}
            onClick={() => onChange(value.filter((_, i) => i !== index))}
          />
        </Group>
      ))}
      {unused.length > 0 && (
        <Group gap={4}>
          <ActionWithTooltip
            tooltip={useTranslate("add")}
            icon={faPlus}
            color="blue.7"
            actionProps={{ size: "sm", variant: "subtle" }}
            iconProps={{ size: "xs" }}
            onClick={() => onChange([...value, { by: unused[0].value, direction: "desc" }])}
          />
          <Text size="xs" c="dimmed">
            {useTranslate("add")}
          </Text>
        </Group>
      )}
    </Stack>
  );
}
