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
 * 条件：非当前账号、未禁用、任意 claude* 模型额度 100% 或已重置。
 * 返回按 quota.last_updated 升序排序（最久没刷新的优先）。
 */
function findSmartRefreshCandidates(accounts: Account[], currentAccountId: string | undefined, sortOldestFirst: boolean, skipLastUpdatedSort = false): Account[] {
  const now = Date.now();
  return accounts
    .filter((acc) => {
      if (acc.id === currentAccountId) return false;
      if (acc.disabled) return false;
      if (!acc.quota?.models?.length) return false;

      // 排除 UNKNOWN 等级帐号，逻辑与 getSubscriptionTier 保持一致：
      // subscription_tier 为空 → UNKNOWN；含 ultra → ULTRA；含 pro → PRO；其他 → FREE
      const rawTier = (acc.quota.subscription_tier ?? '').trim().toLowerCase();
      if (!rawTier) return false; // UNKNOWN
      // ULTRA/PRO/FREE 都是有效等级，不过滤

      return acc.quota.models.some((m) => {
        const name = (m.name || '').toLowerCase();
        if (!name.startsWith('claude')) return false;
        // 额度 100% 或 reset_time 已过期（已重置）
        if (m.percentage === 100) return true;
        if (m.reset_time) {
          const resetDate = new Date(m.reset_time).getTime();
          if (!Number.isNaN(resetDate) && resetDate <= now) return true;
        }
        return false;
      });
    })
    .sort((a, b) => {
      // 非 skipLastUpdatedSort 模式下，先按 last_updated 升序（60秒容差外）
      if (!skipLastUpdatedSort) {
        const aUpdated = a.quota?.last_updated ?? 0;
        const bUpdated = b.quota?.last_updated ?? 0;
        const diff = aUpdated - bUpdated;
        if (Math.abs(diff) > 60) {
          return diff;
        }
      }
      // fallback：按 created_at 排序（方向取决于配置）
      if (sortOldestFirst) {
        return (a.created_at ?? 0) - (b.created_at ?? 0);
      }
      return (b.created_at ?? 0) - (a.created_at ?? 0);
    });
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
  refresh_sort_oldest_first?: boolean;
}

export function useAutoRefresh() {
  const syncCurrentFromClient = useAccountStore((state) => state.syncCurrentFromClient);
  const fetchAccounts = useAccountStore((state) => state.fetchAccounts);
  const fetchCurrentAccount = useAccountStore((state) => state.fetchCurrentAccount);

  const refreshAllCodexQuotas = useCodexAccountStore((state) => state.refreshAllQuotas);
  const fetchCodexAccounts = useCodexAccountStore((state) => state.fetchAccounts);
  const fetchCodexCurrentAccount = useCodexAccountStore((state) => state.fetchCurrentAccount);
  const refreshAllGhcpTokens = useGitHubCopilotAccountStore((state) => state.refreshAllTokens);
  const refreshAllWindsurfTokens = useWindsurfAccountStore((state) => state.refreshAllTokens);
  const refreshAllKiroTokens = useKiroAccountStore((state) => state.refreshAllTokens);

  const agIntervalRef = useRef<number | null>(null);
  const autoSwitchIntervalRef = useRef<number | null>(null);
  const codexIntervalRef = useRef<number | null>(null);
  const ghcpIntervalRef = useRef<number | null>(null);
  const windsurfIntervalRef = useRef<number | null>(null);
  const kiroIntervalRef = useRef<number | null>(null);

  const agRefreshingRef = useRef(false);
  const codexRefreshingRef = useRef(false);
  const ghcpRefreshingRef = useRef(false);
  const windsurfRefreshingRef = useRef(false);
  const kiroRefreshingRef = useRef(false);
  const autoSwitchRefreshingRef = useRef(false);

  const setupRunningRef = useRef(false);
  const setupPendingRef = useRef(false);
  const destroyedRef = useRef(false);
  const lastRefreshTimeRef = useRef<number>(0);

  /**
   * 创建与时钟边界对齐的自调度定时器。
   * 例如 intervalMs = 2min 时，执行时间为 :00:00, :02:00, :04:00 …
   * 每次回调完成后重新计算延迟，自校正无漂移。
   * 返回首次 setTimeout 的 ID（可用 clearTimeout 取消）。
   */
  const scheduleAligned = (
    intervalMs: number,
    callback: () => Promise<void> | void,
    ref: React.MutableRefObject<number | null>,
  ) => {
    const tick = async () => {
      // 窗口隐藏到托盘时跳过本次刷新
      try {
        const visible = await getCurrentWindow().isVisible();
        if (!visible) {
          // 继续调度下一次，但跳过本次执行
          const now = Date.now();
          const next = Math.ceil(now / intervalMs) * intervalMs;
          const delay = next - now || intervalMs;
          ref.current = window.setTimeout(tick, delay);
          return;
        }
      } catch { /* 查询失败时正常执行 */ }

      // 用户空闲超过 10 分钟时跳过本次刷新
      try {
        const idleSeconds = await invoke<number>('get_user_idle_seconds');
        if (idleSeconds >= 600) {
          const now = Date.now();
          const next = Math.ceil(now / intervalMs) * intervalMs;
          const delay = next - now || intervalMs;
          ref.current = window.setTimeout(tick, delay);
          return;
        }
      } catch { /* 查询失败时正常执行 */ }

      await callback();
      if (ref.current === null) return; // 已被清理
      const now = Date.now();
      const next = Math.ceil(now / intervalMs) * intervalMs;
      const delay = next - now || intervalMs;
      ref.current = window.setTimeout(tick, delay);
    };
    const now = Date.now();
    const next = Math.ceil(now / intervalMs) * intervalMs;
    const delay = next - now || intervalMs;
    ref.current = window.setTimeout(tick, delay);
  };

  const clearAllTimers = useCallback(() => {
    const refs = [agIntervalRef, codexIntervalRef, ghcpIntervalRef, autoSwitchIntervalRef, windsurfIntervalRef, kiroIntervalRef];
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

          // 检测配额重置任务状态及唤醒总开关
          const wakeupEnabled = localStorage.getItem('agtools.wakeup.enabled') === 'true';
          if (wakeupEnabled) {
            const tasksJson = localStorage.getItem('agtools.wakeup.tasks');
            if (tasksJson) {
              try {
                const tasks = JSON.parse(tasksJson);
                const hasActiveResetTask = Array.isArray(tasks) && tasks.some(
                  (task: unknown) => {
                    if (!task || typeof task !== 'object') {
                      return false;
                    }
                    const taskObj = task as {
                      enabled?: boolean;
                      schedule?: { wakeOnReset?: boolean };
                    };
                    return Boolean(taskObj.enabled && taskObj.schedule?.wakeOnReset);
                  },
                );

                // 如果有活跃的重置任务，且刷新间隔为禁用(-1)或大于2分钟，则强制修正为2分钟
                if (hasActiveResetTask && (config.auto_refresh_minutes === -1 || config.auto_refresh_minutes > 2)) {
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
                    extraRefreshCount: 20,
                  });
                  config.auto_refresh_minutes = 2;
                }
              } catch (e) {
                console.error('[AutoRefresh] 解析任务列表失败:', e);
              }
            }
          }

          if (destroyedRef.current) {
            return;
          }

          clearAllTimers();

          if (config.auto_refresh_minutes > 0) {
            const extraCount = config.extra_refresh_count ?? 0;
            console.log(`[AutoRefresh] Antigravity 已启用: 每 ${config.auto_refresh_minutes} 分钟（额外刷新: ${extraCount} 个帐号）`);
            const agMs = config.auto_refresh_minutes * 60 * 1000;

            scheduleAligned(agMs, async () => {
              if (agRefreshingRef.current) {
                return;
              }
              agRefreshingRef.current = true;

              try {
                // Step A: 始终刷新当前账号
                try {
                  await invoke('refresh_current_quota');
                  console.log('[AutoRefresh] 当前账号配额已刷新');
                } catch (e) {
                  console.error('[AutoRefresh] 当前账号刷新失败:', e);
                }

                // Step B: 额外刷新候选账号
                if (extraCount > 0) {
                  await fetchAccounts();
                  const currentAccount = useAccountStore.getState().currentAccount;
                  const allAccounts = useAccountStore.getState().accounts;
                  const candidates = findSmartRefreshCandidates(allAccounts, currentAccount?.id, config.refresh_sort_oldest_first ?? false);
                  const toRefresh = candidates.slice(0, extraCount);

                  if (toRefresh.length > 0) {
                    console.log(`[AutoRefresh] 额外刷新 ${toRefresh.length} 个候选账号`);
                    for (const candidate of toRefresh) {
                      try {
                        await invoke('fetch_account_quota', { accountId: candidate.id });
                        console.log(`[AutoRefresh] 候选账号 ${candidate.email} 配额已刷新`);
                      } catch (e) {
                        console.error(`[AutoRefresh] 候选账号 ${candidate.email} 刷新失败:`, e);
                      }
                    }
                  } else {
                    console.log('[AutoRefresh] 无符合条件的候选账号');
                  }
                }

                // Step C: 更新前端 UI
                await fetchAccounts();
                await fetchCurrentAccount();
              } catch (e) {
                console.error('[AutoRefresh] 刷新失败:', e);
              } finally {
                agRefreshingRef.current = false;
              }
            }, agIntervalRef);
          } else {
            console.log('[AutoRefresh] Antigravity 已禁用');
          }

          if (config.codex_auto_refresh_minutes > 0) {
            console.log(`[AutoRefresh] Codex 已启用: 每 ${config.codex_auto_refresh_minutes} 分钟`);
            const codexMs = config.codex_auto_refresh_minutes * 60 * 1000;

            scheduleAligned(codexMs, async () => {
              if (codexRefreshingRef.current) {
                return;
              }
              codexRefreshingRef.current = true;

              try {
                console.log('[AutoRefresh] 触发 Codex 配额刷新...');
                await refreshAllCodexQuotas();
              } catch (e) {
                console.error('[AutoRefresh] Codex 刷新失败:', e);
              } finally {
                codexRefreshingRef.current = false;
              }
            }, codexIntervalRef);
          } else {
            console.log('[AutoRefresh] Codex 已禁用');
          }

          if (config.ghcp_auto_refresh_minutes > 0) {
            console.log(`[AutoRefresh] GitHub Copilot 已启用: 每 ${config.ghcp_auto_refresh_minutes} 分钟`);
            const ghcpMs = config.ghcp_auto_refresh_minutes * 60 * 1000;

            scheduleAligned(ghcpMs, async () => {
              if (ghcpRefreshingRef.current) {
                return;
              }
              ghcpRefreshingRef.current = true;

              try {
                console.log('[AutoRefresh] 触发 GitHub Copilot Token 刷新...');
                await refreshAllGhcpTokens();
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
            console.log(`[AutoRefresh] Windsurf 已启用: 每 ${config.windsurf_auto_refresh_minutes} 分钟`);
            const windsurfMs = config.windsurf_auto_refresh_minutes * 60 * 1000;

            scheduleAligned(windsurfMs, async () => {
              if (windsurfRefreshingRef.current) {
                return;
              }
              windsurfRefreshingRef.current = true;

              try {
                console.log('[AutoRefresh] 触发 Windsurf 配额刷新...');
                await refreshAllWindsurfTokens();
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
            console.log(`[AutoRefresh] Kiro 已启用: 每 ${config.kiro_auto_refresh_minutes} 分钟`);
            const kiroMs = config.kiro_auto_refresh_minutes * 60 * 1000;

            scheduleAligned(kiroMs, async () => {
              if (kiroRefreshingRef.current) {
                return;
              }
              kiroRefreshingRef.current = true;

              try {
                console.log('[AutoRefresh] 触发 Kiro 配额刷新...');
                await refreshAllKiroTokens();
              } catch (e) {
                console.error('[AutoRefresh] Kiro 刷新失败:', e);
              } finally {
                kiroRefreshingRef.current = false;
              }
            }, kiroIntervalRef);
          } else {
            console.log('[AutoRefresh] Kiro 已禁用');
          }

          // 自动切号开启时，额外每 60 秒刷新当前账号（不影响原有配额自动刷新规则）
          if (config.auto_switch_enabled) {
            console.log('[AutoRefresh] 自动切号已启用: 每 60 秒刷新当前账号');
            scheduleAligned(60 * 1000, async () => {
              if (autoSwitchRefreshingRef.current) {
                return;
              }
              autoSwitchRefreshingRef.current = true;

              try {
                await syncCurrentFromClient();
                await invoke('refresh_current_quota');
                await fetchAccounts();
                await fetchCurrentAccount();
              } catch (e) {
                console.error('[AutoRefresh] 自动切号-当前账号刷新失败:', e);
              } finally {
                autoSwitchRefreshingRef.current = false;
              }
            }, autoSwitchIntervalRef);
          } else {
            console.log('[AutoRefresh] 自动切号未启用，跳过 60 秒当前账号刷新');
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
    refreshAllGhcpTokens,
    refreshAllKiroTokens,
    refreshAllWindsurfTokens,
    syncCurrentFromClient,
  ]);

  useEffect(() => {
    const MIN_REFRESH_INTERVAL = 30000; // 30 秒内不重复刷新
    let unlistenAccountsRefresh: UnlistenFn | undefined;
    listen<string>('accounts:refresh', async () => {
      const now = Date.now();
      if (now - lastRefreshTimeRef.current < MIN_REFRESH_INTERVAL) {
        console.log('[AutoRefresh] 跳过重复刷新（距上次刷新不足 30 秒）');
        return;
      }

      lastRefreshTimeRef.current = now;
      await fetchAccounts();
      await fetchCurrentAccount();
    }).then((fn) => {
      unlistenAccountsRefresh = fn;
    });

    return () => {
      if (unlistenAccountsRefresh) unlistenAccountsRefresh();
    };
  }, [fetchAccounts, fetchCurrentAccount]);

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
