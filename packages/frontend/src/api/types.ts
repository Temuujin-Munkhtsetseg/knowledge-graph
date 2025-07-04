import type { GkgEvent, WorkspaceIndexingEvent, ProjectIndexingEvent } from '@gitlab-org/gkg';

export interface ApiClientConfig {
  timeout: number;
  retryAttempts: number;
  retryDelay: number;
}

export interface BaseEventCallbacks {
  onError?: (error: Error) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
}

export interface EventBusCallbacks extends BaseEventCallbacks {
  onEvent?: (event: GkgEvent) => void;
}

export interface WorkspaceIndexCallbacks extends BaseEventCallbacks {
  onWorkspaceEvent?: (event: WorkspaceIndexingEvent) => void;
  onProjectEvent?: (event: ProjectIndexingEvent) => void;
  onComplete?: () => void;
}
