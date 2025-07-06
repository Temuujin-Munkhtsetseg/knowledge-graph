/* eslint-disable max-classes-per-file */
import type { EventSourceMessage } from 'eventsource-parser';
import type {
  ApiContract,
  GkgEvent,
  HttpMethod,
  ServerInfoResponse,
  WorkspaceDeleteBodyRequest,
  WorkspaceDeleteSuccessResponse,
  WorkspaceIndexBodyRequest,
  WorkspaceListSuccessResponse,
} from '@gitlab-org/gkg';
import { withRetry } from './retry-handler';
import { SSEConnection } from './sse';
import type { ApiClientConfig, EventBusCallbacks, WorkspaceIndexCallbacks } from './types';

const endpointPaths = {
  info: '/api/info',
  workspace_list: '/api/workspace/list',
  workspace_delete: '/api/workspace/delete',
  workspace_index: '/api/workspace/index',
  index: '/api/workspace/index',
  events: '/api/events',
  graph_initial: '/api/graph/initial/{workspace_folder_path}/{project_path}',
} satisfies Record<keyof ApiContract, ApiContract[keyof ApiContract]['path']>;

export class ApiError extends Error {
  readonly status: number;

  readonly response?: Response;

  readonly endpoint?: string;

  constructor(message: string, status: number, response?: Response, endpoint?: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.response = response;
    this.endpoint = endpoint;

    Object.setPrototypeOf(this, ApiError.prototype);
  }
}

class HttpClient {
  protected readonly config: ApiClientConfig;

  constructor(config: Partial<ApiClientConfig> = {}) {
    this.config = {
      timeout: 30000,
      retryAttempts: 3,
      retryDelay: 1000,
      ...config,
    };
  }

  async #makeRequest<T>(
    endpoint: string,
    method: HttpMethod,
    options: {
      body?: unknown;
      headers?: HeadersInit;
      timeout?: number;
    } = {},
  ): Promise<T> {
    const operation = async () => {
      const controller = new AbortController();
      const timeoutId = setTimeout(
        () => controller.abort(),
        options.timeout ?? this.config.timeout,
      );

      try {
        const response = await fetch(endpoint, {
          method,
          headers: {
            'Content-Type': 'application/json',
            ...options.headers,
          },
          body: options.body ? JSON.stringify(options.body) : undefined,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
          const errorText = await response.text().catch(() => 'Unknown error');
          throw new ApiError(
            `HTTP ${response.status}: ${errorText}`,
            response.status,
            response,
            endpoint,
          );
        }

        const contentType = response.headers.get('content-type');
        if (contentType?.includes('application/json')) {
          return response.json() as T;
        }
        return response.text() as T;
      } catch (error) {
        clearTimeout(timeoutId);
        if (error instanceof ApiError) {
          throw error;
        }
        throw new ApiError(
          `Request failed: ${error instanceof Error ? error.message : 'Unknown error'}`,
          0,
          undefined,
          endpoint,
        );
      }
    };

    return withRetry(operation, {
      maxAttempts: this.config.retryAttempts,
      initialDelay: this.config.retryDelay,
    });
  }

  protected async get<T>(
    endpoint: string,
    options?: { headers?: HeadersInit; timeout?: number },
  ): Promise<T> {
    return this.#makeRequest<T>(endpoint, 'GET', options);
  }

  protected async post<T>(
    endpoint: string,
    body?: unknown,
    options?: { headers?: HeadersInit; timeout?: number },
  ): Promise<T> {
    return this.#makeRequest<T>(endpoint, 'POST', { body, ...options });
  }

  protected async delete<T>(
    endpoint: string,
    body?: unknown,
    options?: { headers?: HeadersInit; timeout?: number },
  ): Promise<T> {
    return this.#makeRequest<T>(endpoint, 'DELETE', { body, ...options });
  }
}

export class ApiClient extends HttpClient {
  #sseConnection: SSEConnection | null = null;

  async indexWorkspace(
    data: WorkspaceIndexBodyRequest,
    callbacks: WorkspaceIndexCallbacks = {},
  ): Promise<void> {
    this.#sseConnection = new SSEConnection();

    // Start SSE connection without blocking
    this.#sseConnection
      .connect(endpointPaths.events, {
        ...callbacks,
        onEvent: (event: EventSourceMessage) => {
          if (event.data) {
            const gkgEvent = JSON.parse(event.data) as GkgEvent;
            if (gkgEvent.type === 'WorkspaceIndexing') {
              callbacks.onWorkspaceEvent?.(gkgEvent.payload);
              if (gkgEvent.payload.status === 'Completed') {
                callbacks.onComplete?.();
              }
            } else if (gkgEvent.type === 'ProjectIndexing') {
              callbacks.onProjectEvent?.(gkgEvent.payload);
            }
          }
        },
      })
      .catch((error) => {
        callbacks.onError?.(error);
      });

    // Send the indexing request immediately
    await this.post(endpointPaths.index, data);
  }

  async subscribeToEventBus(callbacks: EventBusCallbacks = {}): Promise<() => void> {
    this.#sseConnection = new SSEConnection();
    await this.#sseConnection.connect(endpointPaths.events, {
      ...callbacks,
      onEvent: (event: EventSourceMessage) => {
        if (event.data) {
          const gkgEvent = JSON.parse(event.data) as GkgEvent;
          callbacks.onEvent?.(gkgEvent);
        }
      },
    });

    return () => {
      this.#sseConnection?.disconnect();
    };
  }

  async getServerInfo(): Promise<ServerInfoResponse> {
    return this.get<ServerInfoResponse>(endpointPaths.info);
  }

  async getWorkspaces(): Promise<WorkspaceListSuccessResponse> {
    const response = await this.get<WorkspaceListSuccessResponse>(endpointPaths.workspace_list);

    if (!response || !response.workspaces || !Array.isArray(response.workspaces)) {
      return { workspaces: [] };
    }

    return response;
  }

  async deleteWorkspace(data: WorkspaceDeleteBodyRequest): Promise<WorkspaceDeleteSuccessResponse> {
    return this.delete<WorkspaceDeleteSuccessResponse>(endpointPaths.workspace_delete, data);
  }
}

export const apiClient = new ApiClient();
