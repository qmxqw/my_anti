import type { Account } from '../types/account';
import type { DisplayGroup, GroupSettings } from '../services/groupService';
import {
  calculateGroupQuota,
  calculateOverallQuota,
} from '../services/groupService';
import { getAntigravityGroupResetTimestamp } from '../presentation/platformAccountPresentation';

export type AntigravitySortDirection = 'asc' | 'desc';

export const ANTIGRAVITY_ACCOUNTS_SORT_BY_STORAGE_KEY = 'accountsSortBy';
export const ANTIGRAVITY_ACCOUNTS_SORT_DIRECTION_STORAGE_KEY = 'accountsSortDirection';
export const ANTIGRAVITY_RESET_SORT_PREFIX = 'reset:';
export const DEFAULT_ANTIGRAVITY_SORT_BY = 'overall';
export const DEFAULT_ANTIGRAVITY_SORT_DIRECTION: AntigravitySortDirection = 'desc';

const getAccountQuotas = (account: Account): Record<string, number> => {
  const quotas: Record<string, number> = {};
  if (!account.quota?.models) {
    return quotas;
  }
  for (const model of account.quota.models) {
    quotas[model.name] = model.percentage;
  }
  return quotas;
};

const toDirectionValue = (diff: number, direction: AntigravitySortDirection) =>
  direction === 'desc' ? diff : -diff;

/** 配额相同时按 created_at 次排序（方向由配置决定） */
const compareByCreatedAtSecondary = (
  a: Account,
  b: Account,
  oldestFirst?: boolean,
) => {
  if (oldestFirst) {
    return (a.created_at ?? 0) - (b.created_at ?? 0);
  }
  return (b.created_at ?? 0) - (a.created_at ?? 0);
};

const buildGroupSettings = (displayGroups: DisplayGroup[]): GroupSettings => {
  const settings: GroupSettings = {
    groupMappings: {},
    groupNames: {},
    groupOrder: displayGroups.map((group) => group.id),
    hiddenGroups: [],
    updatedAt: 0,
    updatedBy: 'desktop',
  };

  for (const group of displayGroups) {
    settings.groupNames[group.id] = group.name;
    for (const modelId of group.models) {
      settings.groupMappings[modelId] = group.id;
    }
  }
  return settings;
};

const compareByOverallQuota = (
  a: Account,
  b: Account,
  direction: AntigravitySortDirection,
  secondarySortOldestFirst?: boolean,
) => {
  const aQuota = calculateOverallQuota(getAccountQuotas(a));
  const bQuota = calculateOverallQuota(getAccountQuotas(b));
  const diff = toDirectionValue(bQuota - aQuota, direction);
  if (diff !== 0) return diff;
  // 配额相同时按 created_at 次排序
  return compareByCreatedAtSecondary(a, b, secondarySortOldestFirst);
};

const compareByCreatedAt = (
  a: Account,
  b: Account,
  direction: AntigravitySortDirection,
) => toDirectionValue(b.created_at - a.created_at, direction);

const compareByGroupReset = (
  a: Account,
  b: Account,
  direction: AntigravitySortDirection,
  group: DisplayGroup,
) => {
  const aReset = getAntigravityGroupResetTimestamp(a, group);
  const bReset = getAntigravityGroupResetTimestamp(b, group);
  if (aReset === null && bReset === null) return 0;
  if (aReset === null) return 1;
  if (bReset === null) return -1;
  return toDirectionValue(bReset - aReset, direction);
};

const compareByGroupQuota = (
  a: Account,
  b: Account,
  sortBy: string,
  direction: AntigravitySortDirection,
  displayGroups: DisplayGroup[],
  secondarySortOldestFirst?: boolean,
) => {
  const groupSettings = buildGroupSettings(displayGroups);
  const aGroupQuota = calculateGroupQuota(sortBy, getAccountQuotas(a), groupSettings) ?? 0;
  const bGroupQuota = calculateGroupQuota(sortBy, getAccountQuotas(b), groupSettings) ?? 0;

  if (aGroupQuota !== bGroupQuota) {
    return toDirectionValue(bGroupQuota - aGroupQuota, direction);
  }

  const aOverall = calculateOverallQuota(getAccountQuotas(a));
  const bOverall = calculateOverallQuota(getAccountQuotas(b));
  const overallDiff = toDirectionValue(bOverall - aOverall, direction);
  if (overallDiff !== 0) return overallDiff;
  // 分组配额和总配额都相同时按 created_at 次排序
  return compareByCreatedAtSecondary(a, b, secondarySortOldestFirst);
};

export interface AntigravityAccountSortOptions {
  sortBy: string;
  sortDirection: AntigravitySortDirection;
  displayGroups: DisplayGroup[];
  secondarySortOldestFirst?: boolean;
}

export const normalizeAntigravitySortBy = (sortBy: string | null | undefined) => {
  const value = sortBy?.trim();
  return value ? value : DEFAULT_ANTIGRAVITY_SORT_BY;
};

export const normalizeAntigravitySortDirection = (
  sortDirection: string | null | undefined,
): AntigravitySortDirection => (sortDirection === 'asc' ? 'asc' : 'desc');

export const createAntigravityAccountComparator = ({
  sortBy,
  sortDirection,
  displayGroups,
  secondarySortOldestFirst,
}: AntigravityAccountSortOptions) => {
  const normalizedSortBy = normalizeAntigravitySortBy(sortBy);

  return (a: Account, b: Account) => {
    if (normalizedSortBy === 'created_at') {
      return compareByCreatedAt(a, b, sortDirection);
    }

    if (normalizedSortBy.startsWith(ANTIGRAVITY_RESET_SORT_PREFIX) && displayGroups.length > 0) {
      const targetGroupId = normalizedSortBy.slice(ANTIGRAVITY_RESET_SORT_PREFIX.length);
      const targetGroup = displayGroups.find((group) => group.id === targetGroupId);
      if (targetGroup) {
        return compareByGroupReset(a, b, sortDirection, targetGroup);
      }
    }

    if (
      normalizedSortBy !== 'default' &&
      normalizedSortBy !== 'overall' &&
      displayGroups.length > 0
    ) {
      return compareByGroupQuota(a, b, normalizedSortBy, sortDirection, displayGroups, secondarySortOldestFirst);
    }

    return compareByOverallQuota(a, b, sortDirection, secondarySortOldestFirst);
  };
};
