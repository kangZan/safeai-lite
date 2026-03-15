import { invokeCommand } from './api';
import type { Entity } from '../types/entity';

export interface CreateEntityDto {
  name: string;
  synonyms: string[];
  regexPattern?: string;
  strategy: 'random_replace' | 'empty';
  enabled: boolean;
}

export interface UpdateEntityDto {
  id: string;
  name: string;
  synonyms: string[];
  regexPattern?: string;
  strategy: 'random_replace' | 'empty';
  enabled: boolean;
}

export const entityApi = {
  getAll: () => invokeCommand<Entity[]>('entity_get_all'),
  getBuiltin: () => invokeCommand<Entity[]>('entity_get_builtin'),
  getCustom: () => invokeCommand<Entity[]>('entity_get_custom'),
  toggle: (id: string, enabled: boolean) =>
    invokeCommand<boolean>('entity_toggle', { id, enabled }),
  updateStrategy: (id: string, strategy: string) =>
    invokeCommand<boolean>('entity_update_strategy', { id, strategy }),
  create: (dto: CreateEntityDto) =>
    invokeCommand<Entity>('entity_create', {
      dto: {
        name: dto.name,
        synonyms: dto.synonyms,
        regex_pattern: dto.regexPattern ?? null,
        strategy: dto.strategy,
        enabled: dto.enabled,
      },
    }),
  update: (dto: UpdateEntityDto) =>
    invokeCommand<Entity>('entity_update', {
      dto: {
        id: dto.id,
        name: dto.name,
        synonyms: dto.synonyms,
        regex_pattern: dto.regexPattern ?? null,
        strategy: dto.strategy,
        enabled: dto.enabled,
      },
    }),
  updateSynonyms: (id: string, synonyms: string[]) =>
    invokeCommand<void>('entity_update_synonyms', { id, synonyms }),
  delete: (id: string) => invokeCommand<boolean>('entity_delete', { id }),
};
