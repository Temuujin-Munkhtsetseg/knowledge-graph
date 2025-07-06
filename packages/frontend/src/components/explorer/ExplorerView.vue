<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Network, Search, Filter, Settings } from 'lucide-vue-next';
import { GraphVisualization } from '@/components/graph';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { useWorkspaces } from '@/hooks/api';

interface Props {
  selectedProjectPath?: string | null;
}

const props = defineProps<Props>();

const searchQuery = ref('');
const selectedProject = ref<string | null>(null);

const { data: workspacesData } = useWorkspaces();

const availableProjects = computed(() => {
  if (!workspacesData.value?.workspaces) return [];

  return workspacesData.value.workspaces
    .flatMap(
      (workspace) =>
        workspace.projects?.map((project) => ({
          workspace: workspace.workspace_info,
          project,
          displayName: project.project_path || '',
          workspacePath: workspace.workspace_info?.workspace_folder_path || '',
          projectPath: project.project_path || '',
          isIndexed: project.status?.toLowerCase() === 'indexed',
        })) || [],
    )
    .filter((item) => item.isIndexed);
});

const defaultProject = computed(() => {
  if (availableProjects.value.length === 0) return null;
  return availableProjects.value[0];
});

const currentProject = computed(() => {
  if (!selectedProject.value) return defaultProject.value;
  return (
    availableProjects.value.find((p) => p.projectPath === selectedProject.value) ||
    defaultProject.value
  );
});

const formatProjectName = (path: string) => {
  const parts = path.split('/');
  return parts[parts.length - 1] || path;
};

const selectProject = (projectPath: string) => {
  selectedProject.value = projectPath;
};

// Watch for external project selection
watch(
  () => props.selectedProjectPath,
  (newProjectPath) => {
    if (newProjectPath) {
      selectedProject.value = newProjectPath;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="space-y-6">
    <!-- Explorer Header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <Network class="h-5 w-5 text-foreground" />
        <h2 class="text-lg font-medium text-foreground">Project Explorer</h2>
      </div>

      <div class="flex items-center gap-2">
        <div class="relative">
          <Search
            class="absolute left-2 top-1/2 transform -translate-y-1/2 h-3 w-3 text-muted-foreground"
          />
          <Input
            v-model="searchQuery"
            placeholder="Search projects..."
            class="pl-7 h-8 w-64 text-xs"
          />
        </div>
        <Button variant="ghost" size="sm" class="h-8 w-8 p-0">
          <Filter class="h-3 w-3" />
        </Button>
        <Button variant="ghost" size="sm" class="h-8 w-8 p-0">
          <Settings class="h-3 w-3" />
        </Button>
      </div>
    </div>

    <!-- Project Selection -->
    <div v-if="availableProjects.length > 0" class="space-y-3">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-medium text-foreground">Available Projects</h3>
        <span class="text-xs text-muted-foreground">
          {{ availableProjects.length }} indexed project{{
            availableProjects.length !== 1 ? 's' : ''
          }}
        </span>
      </div>

      <div class="flex flex-wrap gap-2">
        <Button
          v-for="project in availableProjects"
          :key="project.projectPath"
          :variant="currentProject?.projectPath === project.projectPath ? 'default' : 'outline'"
          size="sm"
          class="h-7 text-xs"
          @click="selectProject(project.projectPath)"
        >
          {{ formatProjectName(project.projectPath) }}
        </Button>
      </div>
    </div>

    <!-- Graph Visualization -->
    <div v-if="currentProject" class="space-y-4">
      <GraphVisualization
        :key="currentProject.projectPath"
        :project-path="currentProject.projectPath"
        :workspace-folder-path="currentProject.workspacePath"
      />
    </div>

    <!-- Empty State -->
    <div v-else class="flex items-center justify-center h-64">
      <Card class="max-w-md">
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <Network class="h-5 w-5" />
            No Projects Available
          </CardTitle>
          <CardDescription>
            No indexed projects found. Make sure you have workspaces added and indexed.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p class="text-sm text-muted-foreground">
            Add workspaces from the sidebar and wait for indexing to complete to see project graphs.
          </p>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
