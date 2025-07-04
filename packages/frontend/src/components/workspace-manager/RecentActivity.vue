<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Play, CheckCircle, XCircle, FileText } from 'lucide-vue-next';
import type {
  GkgEvent,
  WorkspaceIndexingEvent,
  ProjectIndexingEvent,
  WorkspaceIndexingStarted,
  ProjectIndexingStarted,
  WorkspaceIndexingCompleted,
  ProjectIndexingCompleted,
  WorkspaceIndexingFailed,
  ProjectIndexingFailed,
} from '@gitlab-org/gkg';
import RecentActivityItem from './RecentActivityItem.vue';

interface Props {
  lastEvent: GkgEvent | null;
}

const props = defineProps<Props>();

const recentEvents = ref<GkgEvent[]>([]);
const MAX_RECENT_EVENTS = 10;

// Watch for new events and add them to recent events
watch(
  () => props.lastEvent,
  (newEvent) => {
    if (newEvent) {
      recentEvents.value.unshift(newEvent);
      if (recentEvents.value.length > MAX_RECENT_EVENTS) {
        recentEvents.value = recentEvents.value.slice(0, MAX_RECENT_EVENTS);
      }
    }
  },
);

// Helper function to extract timestamp from event payload
const getEventTimestamp = (
  payload: WorkspaceIndexingEvent | ProjectIndexingEvent,
  status: string,
): string => {
  if (status === 'Started') {
    const startedEvent = payload as WorkspaceIndexingStarted | ProjectIndexingStarted;
    return startedEvent.started_at;
  }
  if (status === 'Completed') {
    const completedEvent = payload as WorkspaceIndexingCompleted | ProjectIndexingCompleted;
    return completedEvent.completed_at;
  }
  if (status === 'Failed') {
    const failedEvent = payload as WorkspaceIndexingFailed | ProjectIndexingFailed;
    return failedEvent.failed_at;
  }

  return '';
};

// Helper function to format timestamp
const formatTimestamp = (timestamp: string): string => {
  if (!timestamp) {
    return new Date().toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  return new Date(timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
};

// Helper function to format event for display
const formatEventForDisplay = (event: GkgEvent) => {
  console.log('event', event);
  const { payload, type } = event;
  const { status } = payload;
  const eventTime = getEventTimestamp(payload, status);
  const timestamp = formatTimestamp(eventTime);

  if (type === 'WorkspaceIndexing') {
    const workspacePayload = payload as WorkspaceIndexingEvent;
    const workspacePath =
      workspacePayload.workspace_folder_info?.workspace_folder_path || 'Unknown workspace';
    const workspaceName = workspacePath.split('/').pop() || workspacePath;
    return {
      timestamp,
      description: `Workspace "${workspaceName}"`,
      status,
      type: 'workspace',
    };
  }

  if (type === 'ProjectIndexing') {
    const projectPayload = payload as ProjectIndexingEvent;
    const projectPath = projectPayload.project_info?.project_path || 'Unknown project';
    const projectName = projectPath.split('/').pop() || projectPath;
    return {
      timestamp,
      description: `Project "${projectName}"`,
      status,
      type: 'project',
    };
  }

  return {
    timestamp,
    description: 'Unknown event',
    status,
    type: 'unknown',
  };
};

// Helper function to get status color
const getStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case 'started':
      return 'text-blue-600';
    case 'completed':
      return 'text-green-600';
    case 'failed':
      return 'text-red-600';
    default:
      return 'text-muted-foreground';
  }
};

// Helper function to get status icon component
const getStatusIcon = (status: string) => {
  switch (status.toLowerCase()) {
    case 'started':
      return Play;
    case 'completed':
      return CheckCircle;
    case 'failed':
      return XCircle;
    default:
      return FileText;
  }
};

// Pre-computed formatted events to avoid expensive calculations in template
const formattedRecentEvents = computed(() => {
  return recentEvents.value.map((event) => {
    const formatted = formatEventForDisplay(event);
    return {
      event,
      ...formatted,
      statusIcon: getStatusIcon(formatted.status),
      statusColor: getStatusColor(formatted.status),
    };
  });
});
</script>

<template>
  <div v-if="recentEvents.length > 0" class="space-y-3">
    <div class="flex items-center justify-between">
      <h3 class="text-sm font-medium text-foreground">Recent Activity</h3>
      <span class="text-xs text-muted-foreground">
        Last {{ recentEvents.length }} event{{ recentEvents.length !== 1 ? 's' : '' }}
      </span>
    </div>

    <div class="space-y-2">
      <RecentActivityItem
        v-for="(formattedEvent, index) in formattedRecentEvents"
        :key="`${formattedEvent.event.type}-${index}`"
        :formatted-event="formattedEvent"
      />
    </div>
  </div>
</template>
