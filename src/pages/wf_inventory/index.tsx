import { Container, Group, Tabs, Text, Tooltip } from "@mantine/core";
import { useTranslatePages } from "@hooks/useTranslate.hook";
import classes from "./WFInventory.module.css";
import { useHasAlert } from "@hooks/useHasAlert.hook";
import { RivenPanel } from "./Tabs/Rivens";
import { PartsPanel } from "./Tabs/Parts";
import { ModsPanel } from "./Tabs/Mods";
import { SetsPanel } from "./Tabs/Sets";
import { RelicsPanel } from "./Tabs/Relics";
import { ArcanesPanel } from "./Tabs/Arcanes";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import api from "@api/index";
import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";

dayjs.extend(relativeTime);

export default function WfInventoryPage() {
  // Translate general
  const useTranslate = (key: string, context?: { [key: string]: any }, i18Key?: boolean) =>
    useTranslatePages(`wf_inventory.${key}`, { ...context }, i18Key);
  const useTranslateTabs = (key: string, context?: { [key: string]: any }, i18Key?: boolean) => useTranslate(`tabs.${key}`, { ...context }, i18Key);

  const tabs = [
    { label: useTranslateTabs("riven.title"), component: (isActive: boolean) => <RivenPanel isActive={isActive} />, id: "riven" },
    { label: useTranslateTabs("parts.title"), component: (isActive: boolean) => <PartsPanel isActive={isActive} />, id: "parts" },
    { label: useTranslateTabs("mods.title"), component: (isActive: boolean) => <ModsPanel isActive={isActive} />, id: "mods" },
    { label: useTranslateTabs("sets.title"), component: (isActive: boolean) => <SetsPanel isActive={isActive} />, id: "sets" },
    { label: useTranslateTabs("relics.title"), component: (isActive: boolean) => <RelicsPanel isActive={isActive} />, id: "relics" },
    { label: useTranslateTabs("arcanes.title"), component: (isActive: boolean) => <ArcanesPanel isActive={isActive} />, id: "arcanes" },
  ];
  const [activeTab, setActiveTab] = useState(tabs[0].id);

  // The inventory refreshes on its own - AlecaFrame writes a file, the profile
  // source polls - so the page cannot know when it happened without asking.
  // Polling keeps the relative time honest as it ages, too.
  const lastUpdatedQuery = useQuery({
    queryKey: ["wf_inventory_last_updated"],
    queryFn: () => api.wf_inventory.getLastUpdated(),
    refetchInterval: 30_000,
    retry: false,
  });
  const updatedAt = lastUpdatedQuery.data?.updated_at;

  return (
    <Container p={0} fluid className={`${classes.container} ${useHasAlert() ? classes.alert : ""}`}>
      <Group justify="flex-end" className={classes.lastUpdated}>
        {updatedAt ? (
          // The exact time sits in the tooltip: "3 minutes ago" is what you
          // want at a glance, but not what you want when deciding whether a
          // sync actually ran.
          <Tooltip label={dayjs.unix(updatedAt).format("YYYY-MM-DD HH:mm:ss")}>
            <Text size="xs" c="dimmed">
              {useTranslate("last_updated", { when: dayjs.unix(updatedAt).fromNow() })}
            </Text>
          </Tooltip>
        ) : (
          <Text size="xs" c="dimmed">
            {useTranslate("last_updated_never")}
          </Text>
        )}
      </Group>
      <Tabs value={activeTab} onChange={(value) => setActiveTab(value || tabs[0].id)} orientation="vertical">
        <Tabs.List>
          {tabs.map((tab) => (
            <Tabs.Tab value={tab.id} key={tab.id}>
              {tab.label}
            </Tabs.Tab>
          ))}
        </Tabs.List>
        {tabs.map((tab) => (
          <Tabs.Panel value={tab.id} key={tab.id} className={classes.tabPanel}>
            {tab.component(activeTab === tab.id)}
          </Tabs.Panel>
        ))}
      </Tabs>
    </Container>
  );
}
