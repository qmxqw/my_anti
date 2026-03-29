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

const getAccountQuotas = (account: Account, isCurrentAccount = false): Record<string, number> => {
  const quotas: Record<string, number> = {};
  if (!account.quota?.models) {
    return quotas;
  }
  for (const model of account.quota.models) {
    // 当前帐号按余额排序时等效于 100%
    quotas[model.name] = isCurrentAccount ? 100 : model.percentage;
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
  currentAccountId?: string,
) => {
  // 当前帐号始终排在最前面：新帐号优先时等效 2200/1/1，旧帐号优先时等效 0
  const FUTURE_TS = new Date('2200-01-01T00:00:00Z').getTime();
  const aTs = a.id === currentAccountId ? (oldestFirst ? 0 : FUTURE_TS) : (a.created_at ?? 0);
  const bTs = b.id === currentAccountId ? (oldestFirst ? 0 : FUTURE_TS) : (b.created_at ?? 0);
  if (oldestFirst) {
    return aTs - bTs;
  }
  return bTs - aTs;
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
  currentAccountId?: string,
) => {
  const aQuota = calculateOverallQuota(getAccountQuotas(a, a.id === currentAccountId));
  const bQuota = calculateOverallQuota(getAccountQuotas(b, b.id === currentAccountId));
  const diff = toDirectionValue(bQuota - aQuota, direction);
  if (diff !== 0) return diff;
  // 配额相同时按 created_at 次排序
  return compareByCreatedAtSecondary(a, b, secondarySortOldestFirst, currentAccountId);
};

const compareByCreatedAt = (
  a: Account,
  b: Account,
  direction: AntigravitySortDirection,
) => toDirectionValue(b.created_at - a.created_at, direction);

/** 按配额刷新时间（quota.last_updated）排序，无刷新记录的排最后 */
const compareByRefreshedAt = (
  a: Account,
  b: Account,
  direction: AntigravitySortDirection,
) => {
  const aTs = a.quota?.last_updated ?? null;
  const bTs = b.quota?.last_updated ?? null;
  if (aTs === null && bTs === null) return 0;
  if (aTs === null) return 1;   // 无刷新时间的排最后
  if (bTs === null) return -1;
  return toDirectionValue(bTs - aTs, direction);
};

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
  currentAccountId?: string,
) => {
  const groupSettings = buildGroupSettings(displayGroups);
  const aIsCurrent = a.id === currentAccountId;
  const bIsCurrent = b.id === currentAccountId;
  const aGroupQuota = calculateGroupQuota(sortBy, getAccountQuotas(a, aIsCurrent), groupSettings) ?? 0;
  const bGroupQuota = calculateGroupQuota(sortBy, getAccountQuotas(b, bIsCurrent), groupSettings) ?? 0;

  if (aGroupQuota !== bGroupQuota) {
    return toDirectionValue(bGroupQuota - aGroupQuota, direction);
  }

  const aOverall = calculateOverallQuota(getAccountQuotas(a, aIsCurrent));
  const bOverall = calculateOverallQuota(getAccountQuotas(b, bIsCurrent));
  const overallDiff = toDirectionValue(bOverall - aOverall, direction);
  if (overallDiff !== 0) return overallDiff;
  // 分组配额和总配额都相同时按 created_at 次排序
  return compareByCreatedAtSecondary(a, b, secondarySortOldestFirst, currentAccountId);
};

export interface AntigravityAccountSortOptions {
  sortBy: string;
  sortDirection: AntigravitySortDirection;
  displayGroups: DisplayGroup[];
  secondarySortOldestFirst?: boolean;
  /** 当前帐号 ID，按余额排序时该帐号配额等效 100% */
  currentAccountId?: string;
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
  currentAccountId,
}: AntigravityAccountSortOptions) => {
  const normalizedSortBy = normalizeAntigravitySortBy(sortBy);

  return (a: Account, b: Account) => {
    if (normalizedSortBy === 'email') {
      const diff = a.email.localeCompare(b.email)
      return sortDirection === 'asc' ? diff : -diff;
    }

    if (normalizedSortBy === 'created_at') {
      return compareByCreatedAt(a, b, sortDirection);
    }

    if (normalizedSortBy === 'refreshed_at') {
      return compareByRefreshedAt(a, b, sortDirection);
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
      return compareByGroupQuota(a, b, normalizedSortBy, sortDirection, displayGroups, secondarySortOldestFirst, currentAccountId);
    }

    return compareByOverallQuota(a, b, sortDirection, secondarySortOldestFirst, currentAccountId);
  };
};
