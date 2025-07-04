<script setup lang="ts">
import { computed } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import type { TSProjectInfo } from '@gitlab-org/gkg';
import WorkspaceListItemHeader from './WorkspaceListItemHeader.vue';

interface Props {
  project: TSProjectInfo;
  workspacePath: string;
}

const props = defineProps<Props>();

const formatPath = (path: string) => {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
};

const isError = computed(() => props.project?.status?.toLowerCase() === 'error');
</script>

<template>
  <div class="border border-border bg-card hover:bg-muted/30 transition-colors rounded-sm">
    <div class="flex flex-col space-y-2 p-2">
      <WorkspaceListItemHeader
        :name="formatPath(project?.project_path || 'Unknown project')"
        :status="project?.status || 'unknown'"
        :last-indexed-at="project?.last_indexed_at || null"
        :path="project?.project_path || 'Unknown path'"
        :is-collapsible="false"
      />

      <!-- Error Message with VS Code styling -->
      <div
        v-if="isError && project?.error_message"
        class="ml-4 bg-destructive/5 border border-destructive/20 rounded-sm p-1.5"
      >
        <div class="flex items-start gap-1.5">
          <AlertCircle class="h-3 w-3 flex-shrink-0 mt-0.5 text-destructive" />
          <span class="text-xs text-destructive break-words font-mono">{{
            project?.error_message
          }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
