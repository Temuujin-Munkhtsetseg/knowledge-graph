import { useQuery, useQueryClient, useMutation } from '@tanstack/vue-query';
import { ref } from 'vue';
import type {
  GkgEvent,
  WorkspaceIndexingEvent,
  ProjectIndexingEvent,
  WorkspaceIndexBodyRequest,
  WorkspaceDeleteBodyRequest,
} from '@gitlab-org/gkg';
import { apiClient } from '@/api/client';

export const useServerInfo = () => {
  return useQuery({
    queryKey: ['server-info'],
    queryFn: () => apiClient.getServerInfo(),
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 3,
  });
};

export const useWorkspaces = () => {
  return useQuery({
    queryKey: ['workspaces'],
    queryFn: () => apiClient.getWorkspaces(),
  });
};

export const useDeleteWorkspace = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: WorkspaceDeleteBodyRequest) => apiClient.deleteWorkspace(data),
    onSuccess: () => {
      // eslint-disable-next-line no-void
      void queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
  });
};

export const useIndexWorkspace = () => {
  const queryClient = useQueryClient();
  const isIndexing = ref(false);
  const error = ref<Error | null>(null);

  // Event-based state
  const currentWorkspaceEvent = ref<WorkspaceIndexingEvent | null>(null);
  const currentProjectEvent = ref<ProjectIndexingEvent | null>(null);
  const workspaceEventHistory = ref<WorkspaceIndexingEvent[]>([]);
  const projectEventHistory = ref<ProjectIndexingEvent[]>([]);

  const startIndexing = async (data: WorkspaceIndexBodyRequest) => {
    isIndexing.value = true;
    error.value = null;

    // Reset state
    currentWorkspaceEvent.value = null;
    currentProjectEvent.value = null;
    workspaceEventHistory.value = [];
    projectEventHistory.value = [];

    try {
      await apiClient.indexWorkspace(data, {
        onWorkspaceEvent: (event) => {
          currentWorkspaceEvent.value = event;
          workspaceEventHistory.value.push(event);
        },
        onProjectEvent: (event) => {
          currentProjectEvent.value = event;
          projectEventHistory.value.push(event);
        },
        onError: (err) => {
          error.value = err;
          isIndexing.value = false;
        },
        onComplete: () => {
          isIndexing.value = false;
          // eslint-disable-next-line no-void
          void queryClient.invalidateQueries({ queryKey: ['workspaces'] });
        },
      });
    } catch (err) {
      error.value = err as Error;
      isIndexing.value = false;
    }
  };

  const stopIndexing = () => {
    isIndexing.value = false;
    currentWorkspaceEvent.value = null;
    currentProjectEvent.value = null;
  };

  return {
    startIndexing,
    stopIndexing,
    isIndexing,
    error,
    currentWorkspaceEvent,
    currentProjectEvent,
    workspaceEventHistory,
    projectEventHistory,
  };
};

export const useWorkspaceStream = () => {
  const queryClient = useQueryClient();
  const isConnected = ref(false);
  const lastEvent = ref<GkgEvent | null>(null);

  let cleanup: (() => void) | null = null;

  const startStream = async () => {
    try {
      cleanup = await apiClient.subscribeToEventBus({
        onConnect: () => {
          isConnected.value = true;
        },
        onEvent: (event) => {
          // note - connection_established is a special event that is not a GkgEvent
          // it is not a part of the bindings
          if ((event?.type as string) === 'connection-established') {
            return;
          }

          lastEvent.value = event;

          // Only invalidate queries for workspace/project events
          if (event.type === 'WorkspaceIndexing' || event.type === 'ProjectIndexing') {
            // eslint-disable-next-line no-void
            void queryClient.invalidateQueries({ queryKey: ['workspaces'] });
          }
        },
        onError: () => {
          isConnected.value = false;
          // Don't re-throw the error to prevent it from reaching the error boundary
          // Just log it and update the connection state
        },
        onDisconnect: () => {
          isConnected.value = false;
        },
      });
    } catch (error) {
      isConnected.value = false;
    }
  };

  const stopStream = () => {
    if (cleanup) {
      cleanup();
      cleanup = null;
    }
    isConnected.value = false;
  };

  return {
    startStream,
    stopStream,
    isConnected,
    lastEvent,
  };
};
