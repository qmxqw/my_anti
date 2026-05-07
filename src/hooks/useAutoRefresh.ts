import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAccountStore } from '../stores/useAccountStore';
import { useCodexAccountStore } from '../stores/useCodexAccountStore';
import { useGitHubCopilotAccountStore } from '../stores/useGitHubCopilotAccountStore';
import { useWindsurfAccountStore } from '../stores/useWindsurfAccountStore';
import { useKiroAccountStore } from '../stores/useKiroAccountStore';
import type { Account } from '../types/account';

/**
 * 从账号列表中筛选智能刷新候选账号。
 * 条件：未禁用、任意 claude* 模型 reset_time 已过期（配额已重置）。
 * 返回按 created_at 排序（方向取决于 sortOldestFirst 配置）。
 */
/**
 * 判断账号是否有任意 claude* 模型额度 < 100%（即已被使用过）。
 */
function hasPartialQuota(acc: Account): boolean {
  return (acc.quota?.models ?? []).some((m) => {
    const name = (m.name || '').toLowerCase();
    return name.startsWith('claude') && m.percentage < 100;
  });
}

/**
 * 从账号列表中筛选智能刷新候选账号。
 * 条件：未禁用、任意 claude* 模型 reset_time 已过期（配额已重置）。
 *
 * 返回列表按两段优先级排列：
 *   1. 有任意 claude* 额度 < 100%（已使用过）的账号 —— 按 sortField/sortDesc 排序
 *   2. 所有 claude* 额度均 >= 100%（全满未使用）的账号 —— 按 sortField/sortDesc 排序（补充）
 * 排序字段：sortField = 'created_at'（创建时间）| 'last_used_at'（最近使用时间）
 * 排序方向：sortDesc = false 升序，true 降序
 */
function findSmartRefreshCandidates(
  accounts: Account[],
  _currentAccountId: string | undefined,
  sortField: 'created_at' | 'last_used_at',
  sortDesc: boolean,
  includeFullQuota: boolean,
): Account[] {
  const now = Date.now();

  const sortFn = (a: Account, b: Account) => {
    const va = sortField === 'last_used_at' ? (a.last_used_at ?? 0) : (a.created_at ?? 0);
    const vb = sortField === 'last_used_at' ? (b.last_used_at ?? 0) : (b.created_at ?? 0);
    return sortDesc ? vb - va : va - vb;
  };

  const candidates = accounts.filter((acc) => {
    if (acc.disabled) return false;
    if (!acc.quota?.models?.length) return false;

    // 排除 UNKNOWN 等级帐号，逻辑与 getSubscriptionTier 保持一致：
    // subscription_tier 为空 → UNKNOWN；含 ultra → ULTRA；含 pro → PRO；其他 → FREE
    const rawTier = (acc.quota.subscription_tier ?? '').trim().toLowerCase();
    if (!rawTier) return false; // UNKNOWN
    // ULTRA/PRO/FREE 都是有效等级，不过滤

    // 只刷新配额已重置的帐号（reset_time 已过期）
    return acc.quota.models.some((m) => {
      const name = (m.name || '').toLowerCase();
      if (!name.startsWith('claude')) return false;
      if (m.reset_time) {
        const resetDate = new Date(m.reset_time).getTime();
        if (!Number.isNaN(resetDate) && resetDate <= now) return true;
      }
      return false;
    });
  });

  // 分组：已使用（claude* 额度 < 100%）优先，全满（claude* 额度 >= 100%）按配置决定是否纳入
  const partial = candidates.filter(hasPartialQuota).sort(sortFn);
  // 满额账号按 quota.last_updated（上次刷新时间）升序排：最久未刷新的优先
  // 刷完后 last_updated 更新变大，下次自动轮到其他账号
  const TWENTY_HOURS_MS = 20 * 60 * 60 * 1000;
  const full = candidates.filter((a) => {
    if (hasPartialQuota(a)) return false;
    // 满额账号：距上次刷新不足 20 小时则跳过
    // last_updated 是秒级时间戳（Rust chrono::timestamp()），需 ×1000 转毫秒
    const lastUpdatedMs = (a.quota?.last_updated ?? 0) * 1000;
    return (now - lastUpdatedMs) >= TWENTY_HOURS_MS;
  }).sort(
    (a, b) => (a.quota?.last_updated ?? 0) - (b.quota?.last_updated ?? 0),
  );

  return includeFullQuota ? [...partial, ...full] : [...partial];
}

interface GeneralConfig {
  language: string;
  theme: string;
  auto_refresh_minutes: number;
  codex_auto_refresh_minutes: number;
  ghcp_auto_refresh_minutes: number;
  windsurf_auto_refresh_minutes: number;
  kiro_auto_refresh_minutes: number;
  auto_switch_enabled: boolean;
  close_behavior: string;
  opencode_app_path?: string;
  antigravity_app_path?: string;
  codex_app_path?: string;
  vscode_app_path?: string;
  windsurf_app_path?: string;
  opencode_sync_on_switch?: boolean;
  codex_launch_on_switch?: boolean;
  extra_refresh_count?: number;
  codex_extra_refresh_count?: number;
  ghcp_extra_refresh_count?: number;
  windsurf_extra_refresh_count?: number;
  kiro_extra_refresh_count?: number;
  refresh_sort_oldest_first?: boolean;
  refresh_when_tray?: boolean;
  ui_auto_refresh?: boolean;
  switch_sort_rules?: string;
  switch_created_at_desc?: boolean;
  switch_sort_field?: string;
  switch_sort_desc?: boolean;
  refresh_include_full?: boolean;
  refresh_fallback_current?: boolean;
}

export function useAutoRefresh() {
  const syncCurrentFromClient = useAccountStore((state) => state.syncCurrentFromClient);
  const fetchAccounts = useAccountStore((state) => state.fetchAccounts);
  const fetchCurrentAccount = useAccountStore((state) => state.fetchCurrentAccount);

  const refreshAllCodexQuotas = useCodexAccountStore((state) => state.refreshAllQuotas);
  const refreshCodexQuota = useCodexAccountStore((state) => state.refreshQuota);
  const fetchCodexAccounts = useCodexAccountStore((state) => state.fetchAccounts);
  const fetchCodexCurrentAccount = useCodexAccountStore((state) => state.fetchCurrentAccount);
  const refreshAllGhcpTokens = useGitHubCopilotAccountStore((state) => state.refreshAllTokens);
  const refreshGhcpToken = useGitHubCopilotAccountStore((state) => state.refreshToken);
  const fetchGhcpAccounts = useGitHubCopilotAccountStore((state) => state.fetchAccounts);
  const refreshAllWindsurfTokens = useWindsurfAccountStore((state) => state.refreshAllTokens);
  const refreshWindsurfToken = useWindsurfAccountStore((state) => state.refreshToken);
  const fetchWindsurfAccounts = useWindsurfAccountStore((state) => state.fetchAccounts);
  const refreshAllKiroTokens = useKiroAccountStore((state) => state.refreshAllTokens);
  const refreshKiroToken = useKiroAccountStore((state) => state.refreshToken);
  const fetchKiroAccounts = useKiroAccountStore((state) => state.fetchAccounts);

  const agIntervalRef = useRef<number | null>(null);
  const codexIntervalRef = useRef<number | null>(null);
  const ghcpIntervalRef = useRef<number | null>(null);
  const windsurfIntervalRef = useRef<number | null>(null);
  const kiroIntervalRef = useRef<number | null>(null);

  const agRefreshingRef = useRef(false);
  const codexRefreshingRef = useRef(false);
  const ghcpRefreshingRef = useRef(false);
  const windsurfRefreshingRef = useRef(false);
  const kiroRefreshingRef = useRef(false);

  const setupRunningRef = useRef(false);
  const setupPendingRef = useRef(false);
  const destroyedRef = useRef(false);
  const lastRefreshTimeRef = useRef<number>(0);

  // 独立的唤醒任务配置修正副作用
  useEffect(() => {
    let unmounted = false;

    const checkAndFixInterval = async () => {
      try {
        const wakeupEnabled = localStorage.getItem('agtools.wakeup.enabled') === 'true';
        if (!wakeupEnabled) return;

        const tasksJson = localStorage.getItem('agtools.wakeup.tasks');
        if (!tasksJson) return;

        const tasks = JSON.parse(tasksJson);
        const hasActiveResetTask = Array.isArray(tasks) && tasks.some(
          (task: unknown) => {
            if (!task || typeof task !== 'object') return false;
            const taskObj = task as { enabled?: boolean; schedule?: { wakeOnReset?: boolean } };
            return Boolean(taskObj.enabled && taskObj.schedule?.wakeOnReset);
          }
        );

        if (!hasActiveResetTask) return;

        const config = await invoke<GeneralConfig>('get_general_config');
        if (config.auto_refresh_minutes === -1 || config.auto_refresh_minutes > 2) {
          console.log(`[AutoRefresh] 检测到活跃的配额重置任务，自动修正刷新间隔: ${config.auto_refresh_minutes} -> 2`);

          await invoke('save_general_config', {
            language: config.language,
            theme: config.theme,
            autoRefreshMinutes: 2,
            codexAutoRefreshMinutes: config.codex_auto_refresh_minutes,
            ghcpAutoRefreshMinutes: config.ghcp_auto_refresh_minutes,
            windsurfAutoRefreshMinutes: config.windsurf_auto_refresh_minutes,
            kiroAutoRefreshMinutes: config.kiro_auto_refresh_minutes,
            closeBehavior: config.close_behavior || 'ask',
            opencodeAppPath: config.opencode_app_path ?? '',
            antigravityAppPath: config.antigravity_app_path ?? '',
            codexAppPath: config.codex_app_path ?? '',
            vscodeAppPath: config.vscode_app_path ?? '',
            windsurfAppPath: config.windsurf_app_path ?? '',
            opencodeSyncOnSwitch: config.opencode_sync_on_switch ?? true,
            codexLaunchOnSwitch: config.codex_launch_on_switch ?? true,
            extraRefreshCount: config.extra_refresh_count ?? 20,
            refreshSortOldestFirst: config.refresh_sort_oldest_first ?? false,
          });

          if (!unmounted) {
            window.dispatchEvent(new Event('config-updated'));
          }
        }
      } catch (err) {
        console.error('[AutoRefresh] 检测和修正配额重置任务刷新间隔失败:', err);
      }
    };

    // 初始化检测
    void checkAndFixInterval();

    // 监听任务改变
    const handleStorage = (e: StorageEvent) => {
      if (e.key === 'agtools.wakeup.tasks' || e.key === 'agtools.wakeup.enabled') {
        void checkAndFixInterval();
      }
    };
    window.addEventListener('storage', handleStorage);
    window.addEventListener('wakeup-tasks-updated', checkAndFixInterval);

    return () => {
      unmounted = true;
      window.removeEventListener('storage', handleStorage);
      window.removeEventListener('wakeup-tasks-updated', checkAndFixInterval);
    };
  }, []);

  /**
   * 创建与时钟边界对齐的自调度定时器。
   * 例如 intervalMs = 2min 时，执行时间为 :00:00, :02:00, :04:00 …
   * 每次回调完成后重新计算延迟，自校正无漂移。
   * 返回首次 setTimeout 的 ID（可用 clearTimeout 取消）。
   */
  const scheduleAligned = (
    intervalMs: number,
    callback: () => Promise<number | void> | number | void,
    ref: React.MutableRefObject<number | null>,
    skipTrayCheck = false,
  ) => {
    const scheduleNext = (nextMs: number) => {
      const now = Date.now();
      const next = Math.ceil(now / nextMs) * nextMs;
      const delay = next - now || nextMs;
      ref.current = window.setTimeout(tick, delay);
    };

    const tick = async () => {
      // 窗口隐藏到托盘时跳过本次刷新（可通过配置关闭此行为）
      if (!skipTrayCheck) {
        try {
          const visible = await getCurrentWindow().isVisible();
          if (!visible) {
            scheduleNext(intervalMs);
            return;
          }
        } catch { /* 查询失败时正常执行 */ }
      }

      // 用户空闲超过 10 分钟时跳过本次刷新
      try {
        const idleSeconds = await invoke<number>('get_user_idle_seconds');
        if (idleSeconds >= 600) {
          scheduleNext(intervalMs);
          return;
        }
      } catch { /* 查询失败时正常执行 */ }

      const result = await callback();
      if (ref.current === null) return; // 已被清理
      // 回调返回正数时用作下一次间隔，否则沿用默认间隔
      const nextInterval = (typeof result === 'number' && result > 0) ? result : intervalMs;
      scheduleNext(nextInterval);
    };
    scheduleNext(intervalMs);
  };

  const clearAllTimers = useCallback(() => {
    const refs = [agIntervalRef, codexIntervalRef, ghcpIntervalRef, windsurfIntervalRef, kiroIntervalRef];
    for (const ref of refs) {
      if (ref.current) {
        window.clearTimeout(ref.current);
        ref.current = null;
      }
    }
  }, []);

  const setupAutoRefresh = useCallback(async () => {
    if (destroyedRef.current) {
      return;
    }

    if (setupRunningRef.current) {
      setupPendingRef.current = true;
      return;
    }

    setupRunningRef.current = true;

    try {
      do {
        setupPendingRef.current = false;

        try {
          const config = await invoke<GeneralConfig>('get_general_config');
          if (destroyedRef.current) {
            return;
          }

          if (destroyedRef.current) {
            return;
          }

          clearAllTimers();

          if (config.auto_refresh_minutes > 0) {
            const traySkip = config.refresh_when_tray ?? false;
            console.log(`[AutoRefresh] Antigravity 已启用: 每 ${config.auto_refresh_minutes} 分钟（动态间隔，托盘跳过: ${traySkip}）`);
            const agMs = config.auto_refresh_minutes * 60 * 1000;
            const FAST_INTERVAL = 60_000; // 有候选正常刷新后 1 分钟再来

            scheduleAligned(agMs, async (): Promise<number | void> => {
              if (agRefreshingRef.current) {
                return;
              }
              agRefreshingRef.current = true;

              try {
                // 每次定时刷新时更新当前帐号的 last_used_at
                await invoke('touch_current_last_used').catch(() => { });

                // 静默获取帐号列表用于筛选，不触发 store 更新（避免 UI 闪动）
                const allAccounts = await invoke<Account[]>('list_accounts');
                const currentAccount = useAccountStore.getState().currentAccount;
                // 读取快速切号的排序配置（field + desc 两维）
                const sortField = (config.switch_sort_field === 'last_used_at' ? 'last_used_at' : 'created_at') as 'created_at' | 'last_used_at';
                const sortDesc = config.switch_sort_desc ?? false;
                const includeFullQuota = config.refresh_include_full ?? false;
                const fallbackCurrent = config.refresh_fallback_current ?? false;
                const candidates = findSmartRefreshCandidates(allAccounts, currentAccount?.id, sortField, sortDesc, includeFullQuota);
                // 候选列表为空时，按配置决定是否用当前账号保底
                const toRefresh = candidates.length > 0
                  ? candidates.slice(0, 1)
                  : (fallbackCurrent && currentAccount ? [currentAccount] : []);

                if (toRefresh.length > 0) {
                  const isFallback = candidates.length === 0;
                  console.log(isFallback
                    ? `[AutoRefresh] 无候选账号，当前号保底刷新: ${currentAccount?.email}`
                    : `[AutoRefresh] 刷新已重置账号（满额也刷新: ${config.refresh_include_full ?? false}）`);
                  for (const candidate of toRefresh) {
                    try {
                      await invoke('fetch_account_quota', { accountId: candidate.id });
                      console.log(`[AutoRefresh] 账号 ${candidate.email} 配额已刷新`);
                    } catch (e) {
                      console.error(`[AutoRefresh] 账号 ${candidate.email} 刷新失败:`, e);
                    }
                  }
                  // 有账号额度被刷新时，如果开了自动切号，先执行切号检查再更新 UI
                  if (config.auto_switch_enabled) {
                    try {
                      await syncCurrentFromClient();
                      await invoke('refresh_current_quota');
                    } catch (e) {
                      console.error('[AutoRefresh] 自动切号检查失败:', e);
                    }
                  }
                  // 更新前端 UI
                  await fetchAccounts();
                  await fetchCurrentAccount();
                  // 动态间隔：保底刷新 → 设置间隔，正常候选 → 1 分钟
                  const nextMs = isFallback ? agMs : FAST_INTERVAL;
                  console.log(`[AutoRefresh] 下次刷新间隔: ${nextMs / 1000}s（${isFallback ? '保底' : '正常候选'}）`);
                  return nextMs;
                } else {
                  console.log(`[AutoRefresh] 无配额已重置的候选账号，下次刷新间隔: ${agMs / 1000}s`);
                  return agMs;
                }
              } catch (e) {
                console.error('[AutoRefresh] 刷新失败:', e);
              } finally {
                agRefreshingRef.current = false;
              }
            }, agIntervalRef, traySkip);
          } else {
            console.log('[AutoRefresh] Antigravity 已禁用');
          }

          if (config.codex_auto_refresh_minutes > 0) {
            console.log(`[AutoRefresh] Codex 已启用: 每 ${config.codex_auto_refresh_minutes} 分钟（动态间隔）`);
            const codexMs = config.codex_auto_refresh_minutes * 60 * 1000;
            const CODEX_FAST_INTERVAL = 60_000;

            // skipTrayCheck=true: 窗口不可见时也刷新
            scheduleAligned(codexMs, async (): Promise<number | void> => {
              if (codexRefreshingRef.current) {
                return;
              }
              codexRefreshingRef.current = true;

              try {
                // 筛选候选账号：额度 < 100% 且至少一个窗口的 reset_time 已过期
                const nowMs = Date.now();
                const nowSec = Math.floor(nowMs / 1000);
                const state = useCodexAccountStore.getState();
                const currentAccount = state.currentAccount;

                const candidates = [...state.accounts].filter((acc) => {
                  const q = (acc as { quota?: { hourly_percentage: number; weekly_percentage: number; hourly_reset_time?: number; weekly_reset_time?: number } }).quota;
                  if (!q) return false;
                  const minPct = Math.min(q.hourly_percentage, q.weekly_percentage);
                  if (minPct >= 100) return false; // 额度已满，无需刷新
                  // 至少一个窗口的 reset_time 已过期（timestamp 单位：秒）
                  const hourlyExpired = q.hourly_reset_time != null && q.hourly_reset_time <= nowSec;
                  const weeklyExpired = q.weekly_reset_time != null && q.weekly_reset_time <= nowSec;
                  return hourlyExpired || weeklyExpired;
                }).sort((a, b) => {
                  // 按 last_updated 升序（最久未刷新的优先）
                  const la = (a as { quota?: { last_updated?: number } }).quota?.last_updated ?? 0;
                  const lb = (b as { quota?: { last_updated?: number } }).quota?.last_updated ?? 0;
                  return la - lb;
                });

                // 候选为空时，用当前账号保底
                const toRefresh = candidates.length > 0
                  ? candidates.slice(0, 1)
                  : (currentAccount ? [currentAccount] : []);

                if (toRefresh.length > 0) {
                  const isFallback = candidates.length === 0;
                  console.log(isFallback
                    ? `[AutoRefresh] Codex 无候选账号，当前账号保底: ${currentAccount?.email}`
                    : `[AutoRefresh] Codex 刷新已重置账号（共 ${candidates.length} 个候选）`);
                  for (const acc of toRefresh) {
                    try {
                      await refreshCodexQuota((acc as { id: string }).id);
                      console.log(`[AutoRefresh] Codex 账号 ${(acc as { email: string }).email} 配额已刷新`);
                    } catch (e) {
                      console.error(`[AutoRefresh] Codex 账号 ${(acc as { email: string }).email} 刷新失败:`, e);
                    }
                  }
                  await fetchCodexAccounts();
                  await fetchCodexCurrentAccount();
                  const nextMs = isFallback ? codexMs : CODEX_FAST_INTERVAL;
                  console.log(`[AutoRefresh] Codex 下次刷新间隔: ${nextMs / 1000}s（${isFallback ? '保底' : '正常候选'}）`);
                  return nextMs;
                } else {
                  console.log(`[AutoRefresh] Codex 无候选账号且无当前账号，下次刷新间隔: ${codexMs / 1000}s`);
                  return codexMs;
                }
              } catch (e) {
                console.error('[AutoRefresh] Codex 刷新失败:', e);
              } finally {
                codexRefreshingRef.current = false;
              }
            }, codexIntervalRef, true);
          } else {
            console.log('[AutoRefresh] Codex 已禁用');
          }

          if (config.ghcp_auto_refresh_minutes > 0) {
            console.log(`[AutoRefresh] GitHub Copilot 已启用: 每 ${config.ghcp_auto_refresh_minutes} 分钟（动态间隔）`);
            const ghcpMs = config.ghcp_auto_refresh_minutes * 60 * 1000;
            const GHCP_FAST_INTERVAL = 60_000;

            scheduleAligned(ghcpMs, async (): Promise<number | void> => {
              if (ghcpRefreshingRef.current) {
                return;
              }
              ghcpRefreshingRef.current = true;

              try {
                const allAccounts = useGitHubCopilotAccountStore.getState().accounts;
                const toRefresh = allAccounts.slice(0, 1);
                if (toRefresh.length > 0) {
                  console.log('[AutoRefresh] 触发 GitHub Copilot Token 刷新...');
                  for (const acc of toRefresh) {
                    try {
                      await refreshGhcpToken((acc as { id: string }).id);
                    } catch (e) {
                      console.error('[AutoRefresh] GHCP 账号刷新失败:', e);
                    }
                  }
                  await fetchGhcpAccounts();
                  return GHCP_FAST_INTERVAL;
                } else {
                  return ghcpMs;
                }
              } catch (e) {
                console.error('[AutoRefresh] GitHub Copilot 刷新失败:', e);
              } finally {
                ghcpRefreshingRef.current = false;
              }
            }, ghcpIntervalRef);
          } else {
            console.log('[AutoRefresh] GitHub Copilot 已禁用');
          }

          if (config.windsurf_auto_refresh_minutes > 0) {
            console.log(`[AutoRefresh] Windsurf 已启用: 每 ${config.windsurf_auto_refresh_minutes} 分钟（动态间隔）`);
            const windsurfMs = config.windsurf_auto_refresh_minutes * 60 * 1000;
            const WINDSURF_FAST_INTERVAL = 60_000;

            scheduleAligned(windsurfMs, async (): Promise<number | void> => {
              if (windsurfRefreshingRef.current) {
                return;
              }
              windsurfRefreshingRef.current = true;

              try {
                const allAccounts = useWindsurfAccountStore.getState().accounts;
                const toRefresh = allAccounts.slice(0, 1);
                if (toRefresh.length > 0) {
                  console.log('[AutoRefresh] 触发 Windsurf 配额刷新...');
                  for (const acc of toRefresh) {
                    try {
                      await refreshWindsurfToken((acc as { id: string }).id);
                    } catch (e) {
                      console.error('[AutoRefresh] Windsurf 账号刷新失败:', e);
                    }
                  }
                  await fetchWindsurfAccounts();
                  return WINDSURF_FAST_INTERVAL;
                } else {
                  return windsurfMs;
                }
              } catch (e) {
                console.error('[AutoRefresh] Windsurf 刷新失败:', e);
              } finally {
                windsurfRefreshingRef.current = false;
              }
            }, windsurfIntervalRef);
          } else {
            console.log('[AutoRefresh] Windsurf 已禁用');
          }

          if (config.kiro_auto_refresh_minutes > 0) {
            console.log(`[AutoRefresh] Kiro 已启用: 每 ${config.kiro_auto_refresh_minutes} 分钟（动态间隔）`);
            const kiroMs = config.kiro_auto_refresh_minutes * 60 * 1000;
            const KIRO_FAST_INTERVAL = 60_000;

            scheduleAligned(kiroMs, async (): Promise<number | void> => {
              if (kiroRefreshingRef.current) {
                return;
              }
              kiroRefreshingRef.current = true;

              try {
                const allAccounts = useKiroAccountStore.getState().accounts;
                const toRefresh = allAccounts.slice(0, 1);
                if (toRefresh.length > 0) {
                  console.log('[AutoRefresh] 触发 Kiro 配额刷新...');
                  for (const acc of toRefresh) {
                    try {
                      await refreshKiroToken((acc as { id: string }).id);
                    } catch (e) {
                      console.error('[AutoRefresh] Kiro 账号刷新失败:', e);
                    }
                  }
                  await fetchKiroAccounts();
                  return KIRO_FAST_INTERVAL;
                } else {
                  return kiroMs;
                }
              } catch (e) {
                console.error('[AutoRefresh] Kiro 刷新失败:', e);
              } finally {
                kiroRefreshingRef.current = false;
              }
            }, kiroIntervalRef);
          } else {
            console.log('[AutoRefresh] Kiro 已禁用');
          }

          if (config.auto_switch_enabled) {
            console.log('[AutoRefresh] 自动切号已启用：将在账号额度刷新后自动检查');
          }
        } catch (err) {
          console.error('[AutoRefresh] 加载配置失败:', err);
        }
      } while (setupPendingRef.current && !destroyedRef.current);
    } finally {
      setupRunningRef.current = false;
    }
  }, [
    clearAllTimers,
    fetchAccounts,
    fetchCurrentAccount,
    fetchCodexAccounts,
    fetchCodexCurrentAccount,
    refreshAllCodexQuotas,
    refreshCodexQuota,
    refreshAllGhcpTokens,
    refreshGhcpToken,
    fetchGhcpAccounts,
    refreshAllWindsurfTokens,
    refreshWindsurfToken,
    fetchWindsurfAccounts,
    refreshAllKiroTokens,
    refreshKiroToken,
    fetchKiroAccounts,
    syncCurrentFromClient,
  ]);

  useEffect(() => {
    const MIN_REFRESH_INTERVAL = 30000; // 30 秒内不重复刷新
    let unlistenAccountsRefresh: UnlistenFn | undefined;
    listen<string>('accounts:refresh', async (event) => {
      const isAccountSwitched = event.payload === 'account_switched';
      const now = Date.now();
      // 账号切换事件必须立即处理，不受冷却限制
      if (!isAccountSwitched && now - lastRefreshTimeRef.current < MIN_REFRESH_INTERVAL) {
        console.log('[AutoRefresh] 跳过重复刷新（距上次刷新不足 30 秒）');
        return;
      }

      lastRefreshTimeRef.current = now;
      await fetchAccounts();
      await fetchCurrentAccount();
      // Codex 自动换号后也需同步更新前端
      await fetchCodexAccounts();
      await fetchCodexCurrentAccount();
      if (isAccountSwitched) {
        console.log('[AutoRefresh] 账号切换事件，已强制刷新 AG + Codex 数据');
      }
    }).then((fn) => {
      unlistenAccountsRefresh = fn;
    });

    return () => {
      if (unlistenAccountsRefresh) unlistenAccountsRefresh();
    };
  }, [fetchAccounts, fetchCurrentAccount, fetchCodexAccounts, fetchCodexCurrentAccount]);

  useEffect(() => {
    destroyedRef.current = false;
    void setupAutoRefresh();

    let debounceTimer: number | null = null;
    const handleConfigUpdate = () => {
      if (debounceTimer) window.clearTimeout(debounceTimer);
      debounceTimer = window.setTimeout(() => {
        console.log('[AutoRefresh] 检测到配置变更，重新设置定时器');
        void setupAutoRefresh();
      }, 500);
    };

    window.addEventListener('config-updated', handleConfigUpdate);

    return () => {
      destroyedRef.current = true;
      setupPendingRef.current = false;
      if (debounceTimer) window.clearTimeout(debounceTimer);
      clearAllTimers();
      window.removeEventListener('config-updated', handleConfigUpdate);
    };
  }, [clearAllTimers, setupAutoRefresh]);
}
