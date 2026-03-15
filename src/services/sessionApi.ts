import { invoke } from '@tauri-apps/api/core';
import type { SessionListItem, SessionDetail } from '../types/session';

export const sessionApi = {
  /// 获取所有会话列表（按时间倒序）
  getAll(): Promise<SessionListItem[]> {
    return invoke<SessionListItem[]>('session_get_all');
  },

  /// 获取会话详情（含映射关系）
  getById(id: string): Promise<SessionDetail> {
    return invoke<SessionDetail>('session_get_by_id', { id });
  },

  /// 删除单个会话
  delete(id: string): Promise<boolean> {
    return invoke<boolean>('session_delete', { id });
  },

  /// 清空所有会话
  clearAll(): Promise<boolean> {
    return invoke<boolean>('session_clear_all');
  },
};
