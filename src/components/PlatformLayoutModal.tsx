import { MouseEvent as ReactMouseEvent, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { GripVertical, X } from 'lucide-react';
import { PlatformId } from '../types/platform';
import { usePlatformLayoutStore } from '../stores/usePlatformLayoutStore';
import { getPlatformLabel, renderPlatformIcon } from '../utils/platformMeta';

interface PlatformLayoutModalProps {
  open: boolean;
  onClose: () => void;
}

export function PlatformLayoutModal({ open, onClose }: PlatformLayoutModalProps) {
  const { t } = useTranslation();
  const {
    orderedPlatformIds,
    hiddenPlatformIds,
    sidebarPlatformIds,
    trayPlatformIds,
    movePlatform,
    setHiddenPlatform,
    setSidebarPlatform,
    setTrayPlatform,
    resetPlatformLayout,
  } = usePlatformLayoutStore();
  const [draggingId, setDraggingId] = useState<PlatformId | null>(null);
  const [dropTargetId, setDropTargetId] = useState<PlatformId | null>(null);

  const hiddenSet = useMemo(() => new Set(hiddenPlatformIds), [hiddenPlatformIds]);
  const sidebarSet = useMemo(() => new Set(sidebarPlatformIds), [sidebarPlatformIds]);
  const traySet = useMemo(() => new Set(trayPlatformIds), [trayPlatformIds]);

  const stopDragging = () => {
    setDraggingId(null);
    setDropTargetId(null);
  };

  useEffect(() => {
    if (!open || !draggingId) return;
    const handleMouseUp = () => {
      setDraggingId(null);
      setDropTargetId(null);
    };
    window.addEventListener('mouseup', handleMouseUp);
    return () => window.removeEventListener('mouseup', handleMouseUp);
  }, [open, draggingId]);

  const handleDragStart = (event: ReactMouseEvent, id: PlatformId) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    setDraggingId(id);
    setDropTargetId(null);
  };

  const handleDragMove = (targetId: PlatformId) => {
    if (!draggingId) return;
    if (draggingId === targetId) {
      setDropTargetId(null);
      return;
    }
    setDropTargetId(targetId);
    const fromIndex = orderedPlatformIds.indexOf(draggingId);
    const toIndex = orderedPlatformIds.indexOf(targetId);
    if (fromIndex < 0 || toIndex < 0) return;
    movePlatform(fromIndex, toIndex);
  };

  if (!open) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-lg" onClick={(event) => event.stopPropagation()}>
        <div className="modal-header">
          <h2>{t('platformLayout.title', '平台布局')}</h2>
          <button className="modal-close" onClick={onClose} aria-label={t('common.close', '关闭')}>
            <X />
          </button>
        </div>

        <div className="modal-body platform-layout-modal-body">
          <div className="platform-layout-summary">
            <span>
              {t('platformLayout.sidebarSelected', {
                count: sidebarPlatformIds.length,
                defaultValue: '侧边栏已选择 {{count}} 个',
              })}
            </span>
            <button className="btn btn-secondary" onClick={resetPlatformLayout}>
              {t('platformLayout.reset', '恢复默认')}
            </button>
          </div>

          <div className="platform-layout-tip">
            {t(
              'platformLayout.tip',
              '拖拽可排序；最多选择两个平台显示在侧边栏。若只选择一个，侧边栏只显示一个。',
            )}
          </div>

          <div
            className={`platform-layout-list ${draggingId ? 'is-sorting' : ''}`}
            onMouseUp={stopDragging}
            onMouseLeave={stopDragging}
          >
            {orderedPlatformIds.map((platformId) => {
              const hidden = hiddenSet.has(platformId);
              const selected = sidebarSet.has(platformId);
              const sidebarDisabled = hidden;
              const rowClass = [
                'platform-layout-row',
                hidden ? 'is-hidden' : '',
                draggingId === platformId ? 'is-dragging' : '',
                draggingId && draggingId !== platformId ? 'is-drop-candidate' : '',
                draggingId && draggingId !== platformId && dropTargetId === platformId ? 'is-drop-target' : '',
              ]
                .join(' ')
                .trim();

              return (
                <div
                  key={platformId}
                  className={rowClass}
                  onMouseEnter={() => handleDragMove(platformId)}
                >
                  <div className="platform-layout-main">
                    <button
                      type="button"
                      className="platform-layout-drag-trigger"
                      onMouseDown={(event) => handleDragStart(event, platformId)}
                      aria-label={t('platformLayout.dragHandleLabel', '拖拽排序')}
                    >
                      <GripVertical size={16} className="drag-handle" />
                    </button>
                    <div className="platform-layout-icon">{renderPlatformIcon(platformId, 18)}</div>
                    <span className="platform-layout-name">{getPlatformLabel(platformId, t)}</span>
                  </div>

                  <div className="platform-layout-controls">
                    <label className={`platform-layout-toggle ${sidebarDisabled ? 'is-disabled' : ''}`}>
                      <input
                        type="checkbox"
                        checked={selected}
                        disabled={sidebarDisabled}
                        onChange={(event) => setSidebarPlatform(platformId, event.target.checked)}
                      />
                      <span>{t('platformLayout.sidebarToggle', '侧边栏显示')}</span>
                    </label>

                    <label className="platform-layout-toggle">
                      <input
                        type="checkbox"
                        checked={!hidden}
                        onChange={(event) => setHiddenPlatform(platformId, !event.target.checked)}
                      />
                      <span>{t('platformLayout.dashboardToggle', '仪表盘显示')}</span>
                    </label>

                    <label className="platform-layout-toggle">
                      <input
                        type="checkbox"
                        checked={traySet.has(platformId)}
                        onChange={(event) => setTrayPlatform(platformId, event.target.checked)}
                      />
                      <span>{t('platformLayout.trayToggle', '菜单栏显示')}</span>
                    </label>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
