/**
 * 分组配置服务
 * 与后端 group_settings 模块交互
 */

import { invoke } from '@tauri-apps/api/core';

/** 分组配置 */
export interface GroupSettings {
  groupMappings: Record<string, string>;  // modelId -> groupId
  groupNames: Record<string, string>;     // groupId -> displayName
  groupOrder: string[];                   // 分组排序
  hiddenGroups: string[];                 // 被隐藏的分组 ID 列表
  updatedAt: number;                      // 最后更新时间戳
  updatedBy: 'plugin' | 'desktop';        // 最后更新来源
}

/** 显示用分组信息 */
export interface DisplayGroup {
  id: string;
  name: string;
  models: string[];
}

/**
 * 获取完整分组配置
 */
export async function getGroupSettings(): Promise<GroupSettings> {
  return invoke<GroupSettings>('get_group_settings');
}

/**
 * 保存完整分组配置
 */
export async function saveGroupSettings(
  groupMappings: Record<string, string>,
  groupNames: Record<string, string>,
  groupOrder: string[],
  hiddenGroups: string[] = []
): Promise<void> {
  return invoke('save_group_settings', {
    groupMappings,
    groupNames,
    groupOrder,
    hiddenGroups,
  });
}

/**
 * 设置模型的分组
 */
export async function setModelGroup(modelId: string, groupId: string): Promise<void> {
  return invoke('set_model_group', { modelId, groupId });
}

/**
 * 移除模型的分组
 */
export async function removeModelGroup(modelId: string): Promise<void> {
  return invoke('remove_model_group', { modelId });
}

/**
 * 设置分组名称
 */
export async function setGroupName(groupId: string, name: string): Promise<void> {
  return invoke('set_group_name', { groupId, name });
}

/**
 * 删除分组
 */
export async function deleteGroup(groupId: string): Promise<void> {
  return invoke('delete_group', { groupId });
}

/**
 * 更新分组排序
 */
export async function updateGroupOrder(order: string[]): Promise<void> {
  return invoke('update_group_order', { order });
}

/**
 * 获取显示用分组列表（最多3个）
 */
export async function getDisplayGroups(): Promise<DisplayGroup[]> {
  return invoke<DisplayGroup[]>('get_display_groups');
}

/**
 * 默认分组配置（用于初始化）
 */
export const DEFAULT_GROUP_SETTINGS: GroupSettings = {
  groupMappings: {},
  groupNames: {},
  groupOrder: [],
  hiddenGroups: [],
  updatedAt: 0,
  updatedBy: 'desktop',
};

/**
 * 根据模型配额计算分组配额
 * @param groupId 分组 ID
 * @param modelQuotas 模型配额 { modelId: percentage }
 * @param settings 分组配置
 */
export function calculateGroupQuota(
  groupId: string,
  modelQuotas: Record<string, number>,
  settings: GroupSettings
): number | null {
  const modelsInGroup = Object.entries(settings.groupMappings)
    .filter(([, gid]) => gid === groupId)
    .map(([mid]) => mid);
  
  if (modelsInGroup.length === 0) {
    return null;
  }
  
  let total = 0;
  let count = 0;
  
  for (const modelId of modelsInGroup) {
    if (modelId in modelQuotas) {
      total += modelQuotas[modelId];
      count++;
    }
  }
  
  return count > 0 ? Math.round(total / count) : null;
}

/**
 * 计算账号综合配额
 * @param modelQuotas 模型配额 { modelId: percentage }
 */
export function calculateOverallQuota(modelQuotas: Record<string, number>): number {
  const values = Object.values(modelQuotas);
  if (values.length === 0) return 0;
  return Math.round(values.reduce((a, b) => a + b, 0) / values.length);
}
