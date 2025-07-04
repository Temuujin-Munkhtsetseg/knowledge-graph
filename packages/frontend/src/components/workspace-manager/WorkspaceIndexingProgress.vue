<script setup lang="ts">
import { computed } from 'vue';
import { Loader2, CheckCircle, ChevronDown } from 'lucide-vue-next';
import type { WorkspaceIndexingEvent, ProjectIndexingEvent } from '@gitlab-org/gkg';
import { Badge } from '@/components/ui/badge';
import { Collapsible, CollapsibleContent } from '@/components/ui/collapsible';

interface Props {
  isIndexing: boolean;
  currentWorkspaceEvent: WorkspaceIndexingEvent | null;
  currentProjectEvent: ProjectIndexingEvent | null;
  workspaceEventHistory: WorkspaceIndexingEvent[];
  projectEventHistory: ProjectIndexingEvent[];
  getEventDisplayText: (event: WorkspaceIndexingEvent | ProjectIndexingEvent) => string;
  getEventTimestamp: (event: WorkspaceIndexingEvent | ProjectIndexingEvent) => string;
  formatTime: (timestamp?: string) => string;
}

const props = defineProps<Props>();

const allEventsHistory = computed(() => {
  const events: {
    event: WorkspaceIndexingEvent | ProjectIndexingEvent;
    timestamp: string;
    type: 'workspace' | 'project';
  }[] = [];

  props.workspaceEventHistory.forEach((event) => {
    events.push({
      event,
      timestamp: props.getEventTimestamp(event),
      type: 'workspace',
    });
  });

  props.projectEventHistory.forEach((event) => {
    events.push({
      event,
      timestamp: props.getEventTimestamp(event),
      type: 'project',
    });
  });

  return events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
});

const currentEvent = computed(() => {
  if (props.currentProjectEvent) {
    return {
      event: props.currentProjectEvent,
      timestamp: props.getEventTimestamp(props.currentProjectEvent),
      type: 'project' as const,
    };
  }
  if (props.currentWorkspaceEvent) {
    return {
      event: props.currentWorkspaceEvent,
      timestamp: props.getEventTimestamp(props.currentWorkspaceEvent),
      type: 'workspace' as const,
    };
  }
  return null;
});
</script>

<template>
  <Collapsible :open="isIndexing">
    <CollapsibleContent>
      <div class="border-t border-border">
        <!-- Progress Header -->
        <div class="flex items-center justify-between px-2 py-1.5 bg-muted/20">
          <div class="flex items-center gap-1.5">
            <ChevronDown class="h-3 w-3 text-muted-foreground" />
            <span class="text-xs font-medium text-foreground">Indexing Progress</span>
          </div>
          <div class="flex items-center gap-1">
            <div v-if="isIndexing" class="flex items-center gap-1 text-xs text-muted-foreground">
              <Loader2 class="h-3 w-3 animate-spin" />
              <span class="hidden sm:inline">Live</span>
            </div>
            <Badge v-else variant="outline" class="text-xs h-4 px-1 bg-background">
              <CheckCircle class="h-2 w-2 mr-0.5" />
              Complete
            </Badge>
          </div>
        </div>

        <!-- Current Progress -->
        <div v-if="currentEvent" class="mx-2 mb-2 bg-background border border-border rounded-sm">
          <div class="p-2 space-y-1">
            <div class="flex items-center justify-between gap-2">
              <div class="flex items-center gap-1">
                <span class="text-xs font-medium text-foreground capitalize">
                  {{ currentEvent.type }}
                </span>
                <Badge
                  :variant="currentEvent.event.status === 'Failed' ? 'destructive' : 'secondary'"
                  class="text-xs h-3 px-1"
                >
                  {{ currentEvent.event.status }}
                </Badge>
              </div>
              <span class="text-xs text-muted-foreground flex-shrink-0 font-mono">
                {{ formatTime(currentEvent.timestamp) }}
              </span>
            </div>
            <p class="text-xs text-muted-foreground break-words font-mono">
              {{ getEventDisplayText(currentEvent.event) }}
            </p>
          </div>
        </div>

        <!-- Progress History -->
        <div v-if="allEventsHistory.length > 1" class="px-2 pb-2 space-y-1.5">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-foreground">Event History</span>
            <span class="text-xs text-muted-foreground">
              {{ allEventsHistory.length - 1 }} events
            </span>
          </div>
          <div class="max-h-16 overflow-y-auto space-y-1 pr-1">
            <div
              v-for="(item, index) in allEventsHistory.slice(0, -1)"
              :key="index"
              class="bg-background border border-border rounded-sm p-1.5"
            >
              <div class="flex justify-between items-start gap-2">
                <div class="flex items-center gap-1 min-w-0">
                  <span class="text-xs text-muted-foreground capitalize">
                    {{ item.type }}
                  </span>
                  <Badge
                    :variant="item.event.status === 'Failed' ? 'destructive' : 'secondary'"
                    class="text-xs h-3 px-1"
                  >
                    {{ item.event.status }}
                  </Badge>
                </div>
                <span class="text-xs text-muted-foreground/70 flex-shrink-0 font-mono">
                  {{ formatTime(item.timestamp) }}
                </span>
              </div>
              <p class="text-xs text-muted-foreground break-words font-mono mt-0.5">
                {{ getEventDisplayText(item.event) }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </CollapsibleContent>
  </Collapsible>
</template>
