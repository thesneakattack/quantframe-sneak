import { TauriTypes } from "$types";
import { useMutation } from "@tanstack/react-query";
import { TauriClient } from "..";
export class WfInventoryModule {
  constructor(private readonly client: TauriClient) {}
  async getRivensPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvRivenControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvRivenControllerGetListData>("wf_inventory_get_rivens", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getSyndicatesPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvSyndicateControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvSyndicateControllerGetListData>("wf_inventory_get_syndicates", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getPartsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvPartsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvPartsControllerGetListData>("wf_inventory_get_parts", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getModsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvModsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvModsControllerGetListData>("wf_inventory_get_mods", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getSetsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvSetsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvSetsControllerGetListData>("wf_inventory_get_sets", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getRelicsPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvRelicsControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvRelicsControllerGetListData>("wf_inventory_get_relics", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getArcanesPagination(query: TauriTypes.WFItemControllerGetListParams): Promise<TauriTypes.WFInvArcanesControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.WFInvArcanesControllerGetListData>("wf_inventory_get_arcanes", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getLastUpdated(): Promise<TauriTypes.WFInvLastUpdated> {
    return await this.client.sendInvoke<TauriTypes.WFInvLastUpdated>("wf_inventory_last_updated");
  }
  async getItemDetails(wfmUrl: string, subType?: TauriTypes.SubType): Promise<TauriTypes.StockItem> {
    return await this.client.sendInvoke<TauriTypes.StockItem>("wf_inventory_item_details", {
      wfmUrl,
      subType,
    });
  }
  async refreshPrice(wfmUrl: string, subType?: TauriTypes.SubType, maxRank?: number | null): Promise<TauriTypes.WFInvRefreshedPrice> {
    return await this.client.sendInvoke<TauriTypes.WFInvRefreshedPrice>("wf_inventory_refresh_price", {
      wfmUrl,
      subType,
      maxRank,
    });
  }
   update() {
    return useMutation({
      mutationFn: () => this.client.sendInvoke<void>("wf_inventory_update"),
    });
  }
}
