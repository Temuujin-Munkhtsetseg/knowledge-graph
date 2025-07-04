<script setup lang="ts">
import type {
  GkgEvent,
  WorkspaceIndexingStarted,
  WorkspaceIndexingCompleted,
  WorkspaceIndexingFailed,
  ProjectIndexingFailed,
} from '@gitlab-org/gkg';

interface FormattedEvent {
  event: GkgEvent;
  timestamp: string;
  description: string;
  status: string;
  type: string;
  statusIcon: any;
  statusColor: string;
}

interface Props {
  formattedEvent: FormattedEvent;
}

defineProps<Props>();
</script>

<template>
  <div class="p-3 border border-border bg-card rounded-sm hover:bg-muted/30 transition-colors">
    <div class="flex items-start gap-3">
      <!-- Event Icon and Type -->
      <div class="flex items-center gap-2 min-w-0 flex-1">
        <div class="flex items-center gap-1.5">
          <component
            :is="formattedEvent.statusIcon"
            class="h-3 w-3"
            :class="formattedEvent.statusColor"
          />
          <span class="text-xs font-medium text-foreground capitalize">
            {{ formattedEvent.type }}
          </span>
        </div>

        <!-- Event Description -->
        <div class="min-w-0 flex-1">
          <p class="text-xs text-foreground truncate">
            {{ formattedEvent.description }}
          </p>
        </div>
      </div>

      <!-- Status and Timestamp -->
      <div class="flex items-center gap-2 flex-shrink-0">
        <span class="text-xs font-medium capitalize" :class="formattedEvent.statusColor">
          {{ formattedEvent.status }}
        </span>
        <span class="text-xs text-muted-foreground font-mono">
          {{ formattedEvent.timestamp }}
        </span>
      </div>
    </div>

    <!-- Additional Event Details -->
    <div
      v-if="
        formattedEvent.event.type === 'WorkspaceIndexing' &&
        formattedEvent.event.payload.status === 'Started'
      "
      class="mt-2 pl-6"
    >
      <div class="text-xs text-muted-foreground">
        <span class="font-medium">Projects to process:</span>
        {{
          (formattedEvent.event.payload as WorkspaceIndexingStarted).projects_to_process?.length ||
          0
        }}
      </div>
    </div>

    <div
      v-if="
        formattedEvent.event.type === 'WorkspaceIndexing' &&
        formattedEvent.event.payload.status === 'Completed'
      "
      class="mt-2 pl-6"
    >
      <div class="text-xs text-muted-foreground">
        <span class="font-medium">Projects indexed:</span>
        {{
          (formattedEvent.event.payload as WorkspaceIndexingCompleted).projects_indexed?.length || 0
        }}
      </div>
    </div>

    <div v-if="formattedEvent.event.payload.status === 'Failed'" class="mt-2 pl-6">
      <div
        class="text-xs text-red-600 bg-red-50 dark:bg-red-950/20 p-2 rounded border border-red-200 dark:border-red-800"
      >
        <span class="font-medium">Error:</span>
        {{
          formattedEvent.event.type === 'WorkspaceIndexing'
            ? (formattedEvent.event.payload as WorkspaceIndexingFailed).error
            : (formattedEvent.event.payload as ProjectIndexingFailed).error || 'Unknown error'
        }}
      </div>
    </div>
  </div>
</template>
