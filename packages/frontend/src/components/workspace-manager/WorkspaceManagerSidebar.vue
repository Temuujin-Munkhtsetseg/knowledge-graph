<script setup lang="ts">
import { ref, computed } from 'vue';
import { FolderOpen, Plus, Loader2 } from 'lucide-vue-next';
import type { WorkspaceListSuccessResponse } from '@gitlab-org/gkg';
import WorkspaceList from './WorkspaceList.vue';
import WorkspaceIndexer from './WorkspaceIndexer.vue';
import { Sidebar, SidebarContent, SidebarHeader } from '@/components/ui/sidebar';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent } from '@/components/ui/collapsible';
import { Separator } from '@/components/ui/separator';
import { useWorkspaces } from '@/hooks/api';

const { data: workspacesData, isLoading, error, refetch } = useWorkspaces();
const isIndexerOpen = ref(false);

const emit = defineEmits<{
  openProject: [projectPath: string];
}>();

const workspaces = computed((): WorkspaceListSuccessResponse['workspaces'] => {
  if (workspacesData.value?.workspaces && Array.isArray(workspacesData.value.workspaces)) {
    return workspacesData.value.workspaces.filter(
      (workspace) =>
        workspace && workspace.workspace_info && workspace.workspace_info.workspace_folder_path,
    );
  }
  return [];
});

const hasWorkspaces = computed(() => workspaces.value.length > 0);
</script>

<template>
  <Sidebar class="border-r border-border bg-sidebar">
    <SidebarHeader class="p-2">
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-1.5 min-w-0">
          <FolderOpen class="h-3 w-3 flex-shrink-0 text-sidebar-foreground" />
          <h2 class="text-xs font-medium text-sidebar-foreground truncate">Workspaces</h2>
        </div>
        <Button
          variant="ghost"
          size="sm"
          class="h-5 w-5 p-0 flex-shrink-0 hover:bg-sidebar-accent/60"
          :aria-label="isIndexerOpen ? 'Hide workspace form' : 'Show workspace form'"
          @click="isIndexerOpen = !isIndexerOpen"
        >
          <Plus class="h-3 w-3" :class="{ 'rotate-45': isIndexerOpen }" />
        </Button>
      </div>

      <Collapsible v-model:open="isIndexerOpen">
        <CollapsibleContent>
          <div class="mt-2">
            <WorkspaceIndexer @indexed="refetch" />
          </div>
        </CollapsibleContent>
      </Collapsible>
    </SidebarHeader>

    <Separator class="bg-sidebar-border" />

    <SidebarContent class="p-2">
      <!-- Loading State -->
      <div v-if="isLoading" class="flex items-center justify-center py-6">
        <div class="text-center space-y-1.5">
          <Loader2 class="h-4 w-4 animate-spin mx-auto text-sidebar-foreground/60" />
          <p class="text-xs text-sidebar-foreground/60">Loading workspaces...</p>
        </div>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="text-center py-4 space-y-2">
        <div class="text-xs text-destructive">Failed to load workspaces</div>
        <Button
          variant="outline"
          size="sm"
          class="h-6 text-xs border-sidebar-border hover:bg-sidebar-accent/60"
          @click="refetch"
        >
          Retry
        </Button>
      </div>

      <!-- Empty State -->
      <div v-else-if="!hasWorkspaces" class="text-center py-6 space-y-2">
        <FolderOpen class="h-6 w-6 mx-auto opacity-40 text-sidebar-foreground" />
        <div class="space-y-0.5">
          <p class="text-xs text-sidebar-foreground/80">No workspaces found</p>
          <p class="text-xs text-sidebar-foreground/60">Click + to add your first workspace</p>
        </div>
      </div>

      <!-- Workspace List -->
      <div v-else class="space-y-0.5">
        <WorkspaceList
          :workspaces="workspaces"
          @refresh="refetch"
          @open-project="emit('openProject', $event)"
        />
      </div>
    </SidebarContent>
  </Sidebar>
</template>
