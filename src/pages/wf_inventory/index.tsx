import { Container, Tabs } from "@mantine/core";
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
  return (
    <Container p={0} fluid className={`${classes.container} ${useHasAlert() ? classes.alert : ""}`}>
      <Tabs value={activeTab} onChange={(value) => setActiveTab(value || tabs[0].id)} orientation="vertical">
        <Tabs.List>
          {tabs.map((tab) => (
            <Tabs.Tab value={tab.id} key={tab.id}>
              {tab.label}
            </Tabs.Tab>
          ))}
        </Tabs.List>
        {tabs.map((tab) => (
          <Tabs.Panel value={tab.id} key={tab.id}>
            {tab.component(activeTab === tab.id)}
          </Tabs.Panel>
        ))}
      </Tabs>
    </Container>
  );
}
