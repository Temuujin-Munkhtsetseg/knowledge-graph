/* eslint-disable max-classes-per-file */
import type { EventSourceMessage } from 'eventsource-parser';
import type {
  ApiContract,
  GkgEvent,
  GraphInitialQueryRequest,
  GraphInitialSuccessResponse,
  GraphNeighborsQueryRequest,
  GraphNeighborsSuccessResponse,
  GraphSearchQueryRequest,
  GraphSearchSuccessResponse,
  GraphStatsSuccessResponse,
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
  graph_neighbors:
    '/api/graph/neighbors/{workspace_folder_path}/{project_path}/{node_type}/{node_id}',
  graph_search: '/api/graph/search/{workspace_folder_path}/{project_path}',
  graph_stats: '/api/graph/stats/{workspace_folder_path}/{project_path}',
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
    pathParams?: Record<string, string>,
    queryParams?: Record<string, string | number | null>,
  ): Promise<T> {
    let newEndpoint = endpoint;
    if (pathParams) {
      Object.entries(pathParams).forEach(([key, value]) => {
        const encodedValue = encodeURIComponent(value);
        newEndpoint = newEndpoint.replace(`{${key}}`, encodedValue);
      });
    }
    const url = new URL(newEndpoint, window.location.origin);
    if (queryParams) {
      Object.entries(queryParams).forEach(([key, value]) => {
        url.searchParams.set(key, value?.toString() ?? '');
      });
    }
    return this.#makeRequest<T>(url.toString(), 'GET', options);
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

  async triggerWorkspaceIndex(data: WorkspaceIndexBodyRequest): Promise<void> {
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

  async fetchGraphData(
    workspaceFolderPath: string,
    projectPath: string,
  ): Promise<GraphInitialSuccessResponse> {
    const queryParams: GraphInitialQueryRequest = {
      directory_limit: 20,
      file_limit: 100,
      definition_limit: 500,
      imported_symbol_limit: 50,
    };

    return this.get<GraphInitialSuccessResponse>(
      endpointPaths.graph_initial,
      undefined,
      {
        workspace_folder_path: workspaceFolderPath,
        project_path: projectPath,
      },
      queryParams,
    );
  }

  async fetchNodeNeighbors(
    workspaceFolderPath: string,
    projectPath: string,
    nodeId: string,
    nodeType: string,
    limit: number = 100,
  ): Promise<GraphNeighborsSuccessResponse> {
    const queryParams: GraphNeighborsQueryRequest = {
      limit,
    };

    return this.get<GraphNeighborsSuccessResponse>(
      endpointPaths.graph_neighbors,
      undefined,
      {
        workspace_folder_path: workspaceFolderPath,
        project_path: projectPath,
        node_type: nodeType,
        node_id: nodeId,
      },
      queryParams,
    );
  }

  async searchNodes(
    workspaceFolderPath: string,
    projectPath: string,
    searchTerm: string,
    limit: number = 100,
  ): Promise<GraphSearchSuccessResponse> {
    const queryParams: GraphSearchQueryRequest = {
      search_term: searchTerm,
      limit,
    };

    return this.get<GraphSearchSuccessResponse>(
      endpointPaths.graph_search,
      undefined,
      {
        workspace_folder_path: workspaceFolderPath,
        project_path: projectPath,
      },
      queryParams,
    );
  }

  async fetchGraphStats(
    workspaceFolderPath: string,
    projectPath: string,
  ): Promise<GraphStatsSuccessResponse> {
    return this.get<GraphStatsSuccessResponse>(endpointPaths.graph_stats, undefined, {
      workspace_folder_path: workspaceFolderPath,
      project_path: projectPath,
    });
  }
}

export const apiClient = new ApiClient();
