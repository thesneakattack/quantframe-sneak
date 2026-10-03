import { TauriClient } from "..";
import { TauriTypes } from "../../types";
export class SyndicateItemModule {
  constructor(private readonly client: TauriClient) {}

  async importItems(): Promise<number> {
    return await this.client.sendInvoke<number>("syndicate_item_import_items");
  }

  async getPagination(query: TauriTypes.SyndicateItemControllerGetListParams): Promise<TauriTypes.SyndicateItemControllerGetListData> {
    return await this.client.sendInvoke<TauriTypes.SyndicateItemControllerGetListData>("get_syndicate_item_pagination", {
      query: this.client.convertToTauriQuery(query),
    });
  }

  async getFinancialReport(query: TauriTypes.SyndicateItemControllerGetListParams): Promise<TauriTypes.FinancialReport> {
    return await this.client.sendInvoke<TauriTypes.FinancialReport>("get_syndicate_item_financial_report", {
      query: this.client.convertToTauriQuery(query),
    });
  }

  async getStatusCounts(query: TauriTypes.SyndicateItemControllerGetListParams): Promise<{ [key: string]: number }> {
    return await this.client.sendInvoke<{ [key: string]: number }>("get_syndicate_item_status_counts", {
      query: this.client.convertToTauriQuery(query),
    });
  }
  async getSyndicateCounts(query: TauriTypes.SyndicateItemControllerGetListParams): Promise<{ [key: string]: number }> {
    return await this.client.sendInvoke<{ [key: string]: number }>("get_syndicate_item_syndicate_counts", {
      query: this.client.convertToTauriQuery(query),
    });
  }

  async update(input: TauriTypes.UpdateSyndicateItem): Promise<TauriTypes.SyndicateItem> {
    return await this.client.sendInvoke<TauriTypes.SyndicateItem>("syndicate_item_update", { input });
  }
  async updateMultiple(ids: number[], input: TauriTypes.UpdateSyndicateItem): Promise<TauriTypes.SyndicateItem[]> {
    return await this.client.sendInvoke<TauriTypes.SyndicateItem[]>("syndicate_item_update_multiple", { ids, input });
  }

  async delete(id: number): Promise<TauriTypes.SyndicateItem> {
    return await this.client.sendInvoke<TauriTypes.SyndicateItem>("syndicate_item_delete", { id });
  }

  async deleteMultiple(ids: number[]): Promise<number> {
    return await this.client.sendInvoke<number>("syndicate_item_delete_multiple", { ids });
  }

  async sell(entry: TauriTypes.SellSyndicateItem, by?: string): Promise<TauriTypes.SyndicateItem> {
    return await this.client.sendInvoke<TauriTypes.SyndicateItem>("syndicate_item_sell", { ...entry, by });
  }

  async getById<T = any>(id: number, operations?: string[]): Promise<TauriTypes.SyndicateItem<T>> {
    return await this.client.sendInvoke<TauriTypes.SyndicateItem<T>>("syndicate_item_get_by_id", { id, operations });
  }
  exportJson = async (query: TauriTypes.SyndicateItemControllerGetListParams): Promise<string> => {
    return await this.client.sendInvoke<string>("export_syndicate_item_json", {
      query: this.client.convertToTauriQuery(query),
    });
  };
}
