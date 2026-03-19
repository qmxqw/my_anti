import { QuotaData } from '../types/account';
import {
  AUTH_RECOMMENDED_LABELS,
  getAntigravityDisplayModelsFromQuota,
  getAntigravityModelDisplayName,
} from './antigravityModels';

export const DISPLAY_MODEL_ORDER = [
  ...AUTH_RECOMMENDED_LABELS.map((label) => ({ ids: [label], label })),
];

const MODEL_MATCH_REPLACEMENTS: Record<string, string> = {
  'gemini-3-pro-high': 'gemini-3.1-pro-high',
  'gemini-3-pro-low': 'gemini-3.1-pro-low',
  'claude-sonnet-4-5': 'claude-sonnet-4-6',
  'claude-sonnet-4-5-thinking': 'claude-sonnet-4-6',
  'claude-opus-4-5-thinking': 'claude-opus-4-6-thinking',
};

const normalizeModelForMatch = (value: string): string => {
  const normalized = value.trim().toLowerCase();
  if (!normalized) return '';
  return MODEL_MATCH_REPLACEMENTS[normalized] || normalized;
};

export function matchModelName(modelName: string, target: string): boolean {
  const left = normalizeModelForMatch(modelName);
  const right = normalizeModelForMatch(target);
  if (!left || !right) return false;
  return left === right || left.startsWith(`${right}-`) || right.startsWith(`${left}-`);
}

export function getSubscriptionTier(quota?: QuotaData): string {
  const rawTier = quota?.subscription_tier?.trim();
  if (!rawTier) return 'UNKNOWN';

  const tier = rawTier.toLowerCase();
  // 映射等级名称
  if (tier.includes('ultra')) return 'ULTRA';
  if (tier.includes('pro')) return 'PRO';
  return 'FREE';
}

export function getSubscriptionTierDisplay(quota?: QuotaData): string {
  const rawTier = quota?.subscription_tier?.trim();
  if (rawTier) return rawTier;
  return getSubscriptionTier(quota);
}

export function getAntigravityTierBadge(quota?: QuotaData): {
  tier: string;
  label: string;
  className: string;
} {
  const tier = getSubscriptionTier(quota);
  return {
    tier,
    label: tier,
    className: tier.toLowerCase(),
  };
}

export function getQuotaClass(percentage: number): string {
  if (percentage >= 70) return 'high';
  if (percentage >= 30) return 'medium';
  return 'low';
}

type Translate = (key: string, options?: Record<string, unknown>) => string;

const SUSPICIOUS_RESET_MIN_MINUTES = 299;
const SUSPICIOUS_RESET_MAX_MINUTES = 300;
const FILTER_SUSPICIOUS_LS_KEY = 'agtools.ag.filterSuspiciousResetTime';

export function getFilterSuspiciousResetTime(): boolean {
  try {
    const raw = localStorage.getItem(FILTER_SUSPICIOUS_LS_KEY);
    if (raw === null) return true; // default on
    return raw === 'true';
  } catch {
    return true;
  }
}

export function setFilterSuspiciousResetTime(value: boolean): void {
  try {
    localStorage.setItem(FILTER_SUSPICIOUS_LS_KEY, String(value));
  } catch {
    // ignore
  }
}

export function formatResetTime(resetTime: string, t: Translate, filterSuspicious?: boolean): string {
  if (!resetTime) return '';
  try {
    const reset = new Date(resetTime);
    if (Number.isNaN(reset.getTime())) return '';
    const now = new Date();
    const diffMs = reset.getTime() - now.getTime();
    if (diffMs <= 0) return t('common.shared.quota.resetDone');

    const totalMinutes = diffMs / (1000 * 60);

    // 过滤可信度检测：恰好接近整个配额窗口（299~300 分钟）时视为不可信
    if (filterSuspicious && totalMinutes >= SUSPICIOUS_RESET_MIN_MINUTES && totalMinutes <= SUSPICIOUS_RESET_MAX_MINUTES) {
      return t('common.shared.quota.resetDone');
    }

    const totalMinutesInt = Math.floor(totalMinutes);
    const days = Math.floor(totalMinutesInt / (60 * 24));
    const hours = Math.floor((totalMinutesInt % (60 * 24)) / 60);
    const minutes = totalMinutesInt % 60;

    let parts = [];
    if (days > 0) parts.push(`${days}d`);
    if (hours > 0) parts.push(`${hours}h`);
    if (minutes > 0) parts.push(`${minutes}m`);
    
    if (parts.length === 0) return '<1m';
    
    return parts.join(' ');
  } catch {
    return '';
  }
}

export function formatResetTimeAbsolute(resetTime: string): string {
  if (!resetTime) return '';
  const reset = new Date(resetTime);
  if (Number.isNaN(reset.getTime())) return '';
  const pad = (value: number) => String(value).padStart(2, '0');
  const month = pad(reset.getMonth() + 1);
  const day = pad(reset.getDate());
  const hours = pad(reset.getHours());
  const minutes = pad(reset.getMinutes());
  return `${month}/${day} ${hours}:${minutes}`;
}

export function formatResetTimeDisplay(resetTime: string, t: Translate, filterSuspicious?: boolean): string {
  const resetDone = t('common.shared.quota.resetDone');
  const relative = formatResetTime(resetTime, t, filterSuspicious);
  const absolute = formatResetTimeAbsolute(resetTime);
  if (!relative && !absolute) return '';
  if (relative === resetDone) return relative;
  // If we have both, return "relative (absolute)"
  // If only one, return that one
  if (relative && absolute) {
    return `${relative} (${absolute})`;
  }
  return relative || absolute;
}

export function getDisplayModels(quota?: QuotaData) {
  return getAntigravityDisplayModelsFromQuota(quota);
}

export function getModelShortName(name: string): string {
  return getAntigravityModelDisplayName(name);
}
