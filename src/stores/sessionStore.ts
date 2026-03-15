import { create } from 'zustand';
import { message } from 'antd';
import type {
  ScanResult,
  DesensitizeResult,
  RestoreResult,
  SessionListItem,
  SessionDetail,
  MappingItem,
} from '../types/session';
import { desensitizeApi } from '../services/desensitizeApi';
import { sessionApi } from '../services/sessionApi';

/** 前端维护的可编辑清单项 */
export interface ScanItem {
  /** 列表 key 用的临时 id */
  id: string;
  originalValue: string;
  entityName: string;
  strategy: 'random_replace' | 'empty';
  /** true = 排除（保留原文）， false = 脱敏 */
  excluded: boolean;
  /** 在原文中出现的次数（替换次数） */
  count: number;
}

interface SessionState {
  // 扫描阶段
  scanItems: ScanItem[];
  scanLoading: boolean;

  // 当前脱敏会话状态
  currentSessionId: string | null;
  desensitizedContent: string;
  restoredContent: string;
  mappings: MappingItem[];
  loading: boolean;
  error: string | null;

  // 会话列表状态
  sessions: SessionListItem[];
  sessionsLoading: boolean;

  // 扫描操作
  scan: (content: string) => Promise<ScanResult | null>;
  setScanItems: (items: ScanItem[]) => void;

  // 脱敏操作
  desensitize: (content: string, items: ScanItem[]) => Promise<DesensitizeResult | null>;
  restore: (content: string) => Promise<RestoreResult | null>;
  clear: () => void;

  // 会话列表管理
  fetchSessions: () => Promise<void>;
  loadSession: (id: string) => Promise<SessionDetail | null>;
  deleteSession: (id: string) => Promise<boolean>;
  clearAllSessions: () => Promise<boolean>;
  setCurrentSessionId: (id: string | null) => void;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  scanItems: [],
  scanLoading: false,
  currentSessionId: null,
  desensitizedContent: '',
  restoredContent: '',
  mappings: [],
  loading: false,
  error: null,
  sessions: [],
  sessionsLoading: false,

  scan: async (content: string) => {
    set({ scanLoading: true, error: null });
    try {
      const result = await desensitizeApi.scan({ content });
      const items: ScanItem[] = result.items.map((item, idx) => {
        // 统计原文中出现次数
        let count = 0;
        let pos = 0;
        const val = item.originalValue;
        while ((pos = content.indexOf(val, pos)) !== -1) {
          count++;
          pos += val.length;
        }
        return {
          id: `scan-${idx}-${Date.now()}`,
          originalValue: val,
          entityName: item.entityName,
          strategy: item.strategy,
          excluded: false,
          count: Math.max(count, 1),
        };
      });
      set({ scanItems: items, scanLoading: false });
      return result;
    } catch (err) {
      set({ error: String(err), scanLoading: false });
      message.error('扫描失败: ' + err);
      return null;
    }
  },

  setScanItems: (items: ScanItem[]) => {
    set({ scanItems: items });
  },

  desensitize: async (content: string, items: ScanItem[]) => {
    const activeItems = items.filter((i) => !i.excluded);
    if (activeItems.length === 0) {
      message.warning('没有需要脱敏的项目，请检查清单');
      return null;
    }

    set({ loading: true, error: null });
    const { currentSessionId } = get();
    try {
      const result = await desensitizeApi.execute({
        content,
        items: activeItems.map((i) => ({
          originalValue: i.originalValue,
          entityName: i.entityName,
          strategy: i.strategy,
        })),
        sessionId: currentSessionId ?? undefined,
      });
      // 脱敏完成后，加载完整映射详情
      let mappings: MappingItem[] = [];
      try {
        const detail = await sessionApi.getById(result.sessionId);
        mappings = detail.mappings;
      } catch {
        // 加载映射失败不阻断主流程
      }
      set({
        currentSessionId: result.sessionId,
        desensitizedContent: result.desensitizedContent,
        mappings,
        loading: false,
      });
      get().fetchSessions();
      return result;
    } catch (err) {
      set({ error: String(err), loading: false });
      return null;
    }
  },

  restore: async (content: string) => {
    const { currentSessionId } = get();
    if (!currentSessionId) {
      set({ error: '没有可用的会话' });
      return null;
    }

    set({ loading: true, error: null });
    try {
      const result = await desensitizeApi.restore({
        sessionId: currentSessionId,
        content,
      });
      set({
        restoredContent: result.content,
        loading: false,
      });
      return result;
    } catch (err) {
      set({ error: String(err), loading: false });
      return null;
    }
  },

  clear: () => {
    set({
      scanItems: [],
      currentSessionId: null,
      desensitizedContent: '',
      restoredContent: '',
      mappings: [],
      error: null,
    });
  },

  fetchSessions: async () => {
    set({ sessionsLoading: true });
    try {
      const sessions = await sessionApi.getAll();
      set({ sessions, sessionsLoading: false });
    } catch (err) {
      set({ sessionsLoading: false });
      console.error('获取会话列表失败:', err);
    }
  },

  loadSession: async (id: string) => {
    try {
      const detail = await sessionApi.getById(id);
      set({
        currentSessionId: detail.id,
        desensitizedContent: detail.desensitizedContent,
        mappings: detail.mappings,
        restoredContent: '',
      });
      return detail;
    } catch (err) {
      message.error('加载会话失败: ' + err);
      return null;
    }
  },

  deleteSession: async (id: string) => {
    try {
      const ok = await sessionApi.delete(id);
      if (ok) {
        const { currentSessionId } = get();
        if (currentSessionId === id) {
          set({
            currentSessionId: null,
            desensitizedContent: '',
            restoredContent: '',
            mappings: [],
          });
        }
        await get().fetchSessions();
      }
      return ok;
    } catch (err) {
      message.error('删除会话失败: ' + err);
      return false;
    }
  },

  clearAllSessions: async () => {
    try {
      const ok = await sessionApi.clearAll();
      if (ok) {
        set({
          sessions: [],
          currentSessionId: null,
          desensitizedContent: '',
          restoredContent: '',
          mappings: [],
        });
      }
      return ok;
    } catch (err) {
      message.error('清空会话失败: ' + err);
      return false;
    }
  },

  setCurrentSessionId: (id: string | null) => {
    set({ currentSessionId: id });
  },
}));

