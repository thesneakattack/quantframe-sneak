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
  async resolvePrices(keys: TauriTypes.MarketPriceKey[]): Promise<Record<string, number | null>> {
    return await this.client.sendInvoke<Record<string, number | null>>("wf_inventory_resolve_prices", { keys });
  }
  update() {
    return useMutation({
      mutationFn: () => this.client.sendInvoke<void>("wf_inventory_update"),
    });
  }
}
