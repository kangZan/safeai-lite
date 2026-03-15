// ============== 扫描阶段 ==============

export interface ScanInput {
  content: string;
}

export interface ScanResultItem {
  /** 原文 */
  originalValue: string;
  /** 识别依据（实体名） */
  entityName: string;
  /** 继承自实体配置的默认策略 */
  strategy: 'random_replace' | 'empty';
}

export interface ScanResult {
  items: ScanResultItem[];
}

// ============== 脱敏阶段 ==============

/** 前端传给后端的最终清单项 */
export interface DesensitizeItem {
  originalValue: string;
  entityName: string;
  strategy: 'random_replace' | 'empty';
}

export interface DesensitizeInput {
  content: string;
  /** 用户确认后的脸敏清单（已排除的项不包含在内） */
  items: DesensitizeItem[];
  /** 存在则覆盖旧会话 */
  sessionId?: string;
}

export interface DesensitizeResult {
  sessionId: string;
  originalContent: string;
  desensitizedContent: string;
  mappingCount: number;
  createdAt: string;
}

export interface RestoreInput {
  sessionId: string;
  content: string;
}

export interface RestoreResult {
  content: string;
}

export interface Session {
  id: string;
  name: string;
  originalContent: string;
  desensitizedContent: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

/// 会话列表项（精简字段）
export interface SessionListItem {
  id: string;
  name: string;
  status: string;
  mappingCount: number;
  createdAt: string;
  preview: string;
  desensitizedContent: string;
}

/// 脱敏映射项
export interface MappingItem {
  id: string;
  placeholder: string;
  originalValue: string;
  entityName: string;
}

/// 会话详情（含映射关系）
export interface SessionDetail {
  id: string;
  name: string;
  originalContent: string;
  desensitizedContent: string;
  mappings: MappingItem[];
  createdAt: string;
  updatedAt: string;
}
