<script setup lang="ts">
import { computed } from 'vue';
import { AlertCircle, RotateCcw } from 'lucide-vue-next';
import type { TSProjectInfo, WorkspaceIndexBodyRequest } from '@gitlab-org/gkg';
import WorkspaceListItemHeader from './WorkspaceListItemHeader.vue';
import { Button } from '@/components/ui/button';
import { apiClient } from '@/api/client';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

interface Props {
  project: TSProjectInfo;
  workspacePath: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  openProject: [projectPath: string];
  refresh: [];
}>();

const formatPath = (path: string) => {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
};

const isError = computed(() => props.project?.status?.toLowerCase() === 'error');
const isIndexed = computed(() => props.project?.status?.toLowerCase() === 'indexed');

const handleProjectClick = () => {
  if (isIndexed.value && props.project?.project_path) {
    emit('openProject', props.project.project_path);
  }
};

const handleReindexClick = async () => {
  if (!isIndexed.value || !props.workspacePath) return;
  const payload: WorkspaceIndexBodyRequest = {
    workspace_folder_path: props.workspacePath,
  };
  try {
    await apiClient.triggerWorkspaceIndex(payload);
    emit('refresh');
  } catch (_error) {
    // no-op
  }
};
</script>

<template>
  <div
    class="group/project relative border border-border bg-card hover:bg-muted/30 transition-colors rounded-sm"
    :class="{ 'cursor-pointer': isIndexed }"
    @click="handleProjectClick"
  >
    <div
      v-if="isIndexed"
      class="absolute top-1 right-1 opacity-0 group-hover/project:opacity-100 transition-opacity"
    >
      <Button
        variant="ghost"
        size="sm"
        class="h-5 w-5 p-0 hover:bg-muted/60"
        :aria-label="`Re-index ${formatPath(project?.project_path || 'project')}`"
        @click.stop="handleReindexClick"
      >
        <Tooltip>
          <TooltipTrigger class="cursor-pointer">
            <RotateCcw class="h-3 w-3" />
          </TooltipTrigger>
          <TooltipContent>Re-index</TooltipContent>
        </Tooltip>
      </Button>
    </div>

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
