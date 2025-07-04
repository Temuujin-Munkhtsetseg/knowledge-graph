<script setup lang="ts">
import { ref, computed } from 'vue';
import { FolderPlus, AlertCircle, X } from 'lucide-vue-next';
import type { WorkspaceIndexingEvent, ProjectIndexingEvent } from '@gitlab-org/gkg';
import WorkspaceIndexingProgress from './WorkspaceIndexingProgress.vue';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useIndexWorkspace } from '@/hooks/api';

const emit = defineEmits<{
  indexed: [];
}>();

const workspacePath = ref('');
const {
  startIndexing,
  stopIndexing,
  isIndexing,
  error,
  currentWorkspaceEvent,
  currentProjectEvent,
  workspaceEventHistory,
  projectEventHistory,
} = useIndexWorkspace();

const canIndex = computed(() => workspacePath.value.trim() && !isIndexing.value);
const hasProgress = computed(
  () =>
    currentWorkspaceEvent.value ||
    currentProjectEvent.value ||
    workspaceEventHistory.value.length > 0 ||
    projectEventHistory.value.length > 0,
);

const handleIndex = async () => {
  if (!canIndex.value) return;

  await startIndexing({ workspace_folder_path: workspacePath.value.trim() });
  emit('indexed');
  workspacePath.value = '';
};

const handleStop = () => {
  stopIndexing();
};

const formatTime = (timestamp?: string) => {
  if (!timestamp) return '';
  return new Date(timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
};

// Consolidated utility to get all event display properties
const getEventDisplayProperties = (event: WorkspaceIndexingEvent | ProjectIndexingEvent) => {
  const isWorkspaceEvent = 'workspace_folder_info' in event;
  const { status } = event;

  // Helper to get timestamp based on status
  const getTimestamp = (eventData: any) => {
    switch (status) {
      case 'Started':
        return eventData.started_at;
      case 'Completed':
        return eventData.completed_at;
      case 'Failed':
        return eventData.failed_at;
      default:
        return new Date().toISOString();
    }
  };

  if (isWorkspaceEvent) {
    const workspaceEvent = event as any;
    const timestamp = getTimestamp(workspaceEvent);
    const wsPath = workspaceEvent.workspace_folder_info?.workspace_folder_path;

    let displayText = '';
    switch (status) {
      case 'Started':
        displayText = `Started indexing workspace: ${wsPath}`;
        break;
      case 'Completed':
        displayText = `Completed indexing workspace: ${wsPath}`;
        break;
      case 'Failed':
        displayText = `Failed indexing workspace: ${workspaceEvent.error || 'Unknown error'}`;
        break;
      default:
        displayText = 'Processing workspace...';
    }

    return { displayText, timestamp };
  }

  const projectEvent = event as any;
  const timestamp = getTimestamp(projectEvent);
  const projectPath = projectEvent.project_info?.project_path;

  let displayText = '';
  switch (status) {
    case 'Started':
      displayText = `Started indexing project: ${projectPath}`;
      break;
    case 'Completed':
      displayText = `Completed indexing project: ${projectPath}`;
      break;
    case 'Failed':
      displayText = `Failed indexing project: ${projectEvent.error}`;
      break;
    default:
      displayText = 'Processing project...';
  }

  return { displayText, timestamp };
};

// Backward compatibility helpers
const getEventDisplayText = (event: WorkspaceIndexingEvent | ProjectIndexingEvent): string => {
  return getEventDisplayProperties(event).displayText;
};

const getEventTimestamp = (event: WorkspaceIndexingEvent | ProjectIndexingEvent): string => {
  return getEventDisplayProperties(event).timestamp;
};
</script>

<template>
  <div class="border border-border bg-card rounded-sm">
    <!-- VS Code Style Header - More Compact -->
    <div class="flex items-center gap-2 px-2 py-1.5 border-b border-border bg-muted/30">
      <FolderPlus class="h-3 w-3 text-muted-foreground" />
      <span class="text-xs font-medium text-foreground">Add Workspace</span>
    </div>

    <!-- Form Section with VS Code styling - More Compact -->
    <div class="p-2 space-y-2">
      <div class="space-y-1.5">
        <Input
          v-model="workspacePath"
          placeholder="/path/to/workspace"
          :disabled="isIndexing"
          class="h-6 text-xs font-mono bg-background border-border focus:border-primary focus:ring-1 focus:ring-primary/20"
          :aria-label="'Workspace path'"
          @keydown.enter="handleIndex"
        />

        <div class="flex gap-1.5">
          <Button
            v-if="!isIndexing"
            :disabled="!canIndex"
            size="sm"
            class="flex-1 h-6 text-xs bg-primary hover:bg-primary/90 text-primary-foreground"
            @click="handleIndex"
          >
            <FolderPlus class="h-3 w-3 mr-1" />
            Index
          </Button>

          <Button
            v-else
            variant="outline"
            size="sm"
            class="flex-1 h-6 text-xs border-border hover:bg-muted/60"
            @click="handleStop"
          >
            <X class="h-3 w-3 mr-1" />
            Stop
          </Button>
        </div>
      </div>
    </div>

    <!-- Error State with VS Code styling - More Compact -->
    <div v-if="error" class="mx-2 mb-2 bg-destructive/5 border border-destructive/20 rounded-sm">
      <div class="flex items-start gap-2 p-2">
        <AlertCircle class="h-3 w-3 text-destructive flex-shrink-0 mt-0.5" />
        <div class="space-y-0.5 min-w-0">
          <p class="text-xs font-medium text-destructive">Indexing Error</p>
          <p class="text-xs text-destructive/80 break-words font-mono">
            {{ error.message }}
          </p>
        </div>
      </div>
    </div>

    <!-- Progress Section with VS Code styling - More Compact -->
    <WorkspaceIndexingProgress
      v-if="hasProgress"
      :is-indexing="isIndexing"
      :current-workspace-event="currentWorkspaceEvent"
      :current-project-event="currentProjectEvent"
      :workspace-event-history="workspaceEventHistory"
      :project-event-history="projectEventHistory"
      :get-event-display-text="getEventDisplayText"
      :get-event-timestamp="getEventTimestamp"
      :format-time="formatTime"
    />
  </div>
</template>
