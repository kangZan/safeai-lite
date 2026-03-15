export type EntityType = 'builtin' | 'custom';
export type Strategy = 'random_replace' | 'empty';

export interface Entity {
  id: string;
  name: string;
  entityType: EntityType;
  synonyms: string[];
  regexPattern?: string;
  strategy: Strategy;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}
