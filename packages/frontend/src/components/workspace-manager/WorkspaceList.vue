<script setup lang="ts">
import { computed } from 'vue';
import type { WorkspaceWithProjects } from '@gitlab-org/gkg';
import WorkspaceItem from './WorkspaceItem.vue';
import ProjectItem from './ProjectItem.vue';
import WorkspaceListItemHeader from './WorkspaceListItemHeader.vue';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';

interface Props {
  workspaces: WorkspaceWithProjects[];
}

const props = defineProps<Props>();
const emit = defineEmits<{
  refresh: [];
}>();

const processedWorkspaces = computed(() => {
  return props.workspaces
    .filter((workspace) => workspace && workspace.workspace_info)
    .map((workspace) => {
      const isSingleProject = workspace.projects?.length === 1;

      return {
        ...workspace,
        isSingleProject,
      };
    });
});

const formatPath = (path: string) => {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
};
</script>

<template>
  <div class="space-y-0.5">
    <div
      v-for="(workspace, index) in processedWorkspaces"
      :key="workspace.workspace_info?.workspace_folder_path || `workspace-${index}`"
    >
      <!-- Multi-Project Workspace -->
      <Collapsible v-if="!workspace.isSingleProject" v-slot="{ open }" :default-open="true">
        <CollapsibleTrigger class="w-full group" :aria-expanded="open">
          <WorkspaceItem :workspace="workspace.workspace_info" @refresh="emit('refresh')">
            <template #trigger>
              <WorkspaceListItemHeader
                :name="
                  formatPath(workspace.workspace_info?.workspace_folder_path || 'Unknown workspace')
                "
                :status="workspace.workspace_info?.status || 'unknown'"
                :last-indexed-at="workspace.workspace_info?.last_indexed_at || null"
                :path="workspace.workspace_info?.workspace_folder_path || 'Unknown path'"
                :is-collapsible="true"
                :is-open="open"
              />
            </template>
          </WorkspaceItem>
        </CollapsibleTrigger>

        <CollapsibleContent>
          <div class="mt-1 ml-4 space-y-0.5 border-l-2 border-muted-foreground/20 pl-2">
            <ProjectItem
              v-for="(project, projectIndex) in workspace.projects || []"
              :key="project?.project_hash || `project-${index}-${projectIndex}`"
              :project="project"
              :workspace-path="workspace.workspace_info?.workspace_folder_path || 'Unknown path'"
              @refresh="emit('refresh')"
            />
          </div>
        </CollapsibleContent>
      </Collapsible>

      <!-- Single Project Workspace -->
      <WorkspaceItem v-else :workspace="workspace.workspace_info" @refresh="emit('refresh')">
        <template #trigger>
          <WorkspaceListItemHeader
            :name="formatPath(workspace.projects?.[0]?.project_path || 'Unknown project')"
            :status="workspace.projects?.[0]?.status || 'unknown'"
            :last-indexed-at="workspace.projects?.[0]?.last_indexed_at || null"
            :path="workspace.projects?.[0]?.project_path || 'Unknown path'"
            :is-collapsible="false"
          />
        </template>
      </WorkspaceItem>
    </div>
  </div>
</template>
